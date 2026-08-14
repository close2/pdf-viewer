# 503 — An object reference has two answers, and a `Form` is a control

**Finding.** `doc/todo/31` carried two entries in two places and they were one missing link.
§14.7.5.3's object reference makes a structure element's content "an entire PDF object, such as an
XObject directly or indirectly referenced by a page description or an annotation", and
`viewer_core::AccessibilityNode` took exactly one fact from it: that the element is on this page.
Everything else was empty — no extent, because such an element marks no text, and a generic group
on AT-SPI whatever the object was. Two clauses answer for the annotation half of that sentence.
§12.5.2 makes `/Rect` required and "defining the location of the annotation on the page in default
user space units", so the element has a place; and §14.8.4.7.2's Table 368 makes `Form` "[e]ither
an association between content enclosed by the Form structure element and a corresponding widget
annotation or a mechanism to include a widget annotation in the structure tree", with "Form shall
be used for each PDF widget annotation that belongs to the real content of the document" — one
widget, so §12.7.5's field type is the role. Both now cross, and the state of §12.7.5.2's toggling
buttons with them.

**Date.** 2026-08-14.
**ADR.** [0338](../adr/0338-an-object-reference-has-two-answers.md).
**Touched.** `crates/pdf-model/src/structure.rs` (`annotation_rectangles`, one unit test),
`crates/pdf-model/examples/element_bounds_census.rs` (two new counts and the per-document line),
`crates/viewer-core/src/accessibility.rs` (`Gathered::objects`, `AccessibilityNode::control`, the
`Readback` the page's half of the answer became, `referenced_rectangle`),
`crates/viewer-core/src/viewer.rs` (`referenced_objects`),
`crates/viewer-core/tests/headless.rs` (one fixture, one test),
`crates/viewer-accessibility/src/role.rs` (`form_role`, `form_toggled`, `form_note`, one test),
`crates/viewer-accessibility/src/tree.rs` (the control and its state on the node),
`crates/viewer-accessibility/tests/tree.rs` (two tests),
`crates/viewer-confined/src/protocol/panels.rs` and `protocol.rs` (the control on the wire, and a
`Form` in the round trip), `doc/conformance/ledger.toml` (§12.5.2, §12.7.5, §14.7.5.3,
§14.8.4.7.2, §14.8.5.4.3), `doc/verify.md`, `doc/todo/31-accessibility-host.md`,
`doc/adr/0338-*` (new), this file.

## What the census was worth, which is the part to carry forward

The population was counted before anything was built, over 1245 documents and 166 115 structure
elements. Two numbers decided the shape:

- **All 272 of the corpus's `Form` elements name a widget the field tree reaches** — not most of
  them, all of them, split `Tx` 218, check box 43, radio 6, `Ch` 5. The entry ADR 0214 left open
  was not a long tail.
- **All 272 are also in the placeless population**, which is the whole of `Form` in the by-role
  list. A widget annotation marks no text by nature, so the two todo entries were describing the
  same elements from opposite ends — which is why one change closed both and why neither could
  have been done without the other.

333 of the 1675 placeless elements that state no Table 379 `/BBox` are placed by the annotation
rectangle. 1342 remain, and they state nothing any clause answers.

## And it was checked where the defect lived

`doc/verify.md`'s AT-SPI recipe, the same binary twice with `finish` answering `None` for both
fields. `annotation-button-widget.pdf` is the witness worth remembering because **the document
labels its own answers**: nine `Form` elements, each beside a paragraph reading "Check box,
checked", "Radio button, unselected" and so on. Before, all nine were `panel` — AT-SPI's word for
`Role::Group` — implementing no `Component` at all, so a client asking where a form field is got
an error. After, three `check_box` and six `radio_button` with their `/Rect` as extents, and
`state=checked` on exactly the three the document calls checked or selected. At scale,
`prefilled_f1040.pdf` page 1 went from 0 controls to 104 and from 204 nodes with no `Component`
to 100.

## What the next round should know

- **The actions entry on `doc/todo/31` is now the sharpest thing on it.** A check box that
  announces itself as a check box invites the request this tree declines; `Command::Activate` on
  the widget is the answer and the node already carries the evidence that it is one.
- **`GetState` answers `au` — 32-bit words**, and decoding it as 64-bit prints plausible-looking
  state numbers that name the wrong states. Cost ten minutes; written into `doc/verify.md`.
- **A signature field is refused rather than mapped**, in writing, so it is not re-derived: neither
  AccessKit nor AT-SPI has a role for one, and every candidate asserts something about what a
  person may do with it that this program does not implement.
- No raster can change — nothing on the drawing path was touched, and `Query::AccessibilityTree`
  is read by no rasteriser — so the corpus, oracle and quorra gates were not owed by this change.

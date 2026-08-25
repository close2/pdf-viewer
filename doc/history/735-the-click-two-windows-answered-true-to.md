# 735 — The click two windows answered `true` to

The eleventh UI round, on `doc/todo/31`'s next item. ADR 0630.

## What it took

731 measured a silence and stopped it: a click on a §12.7 widget did nothing in a host that
delegates the widget's appearance, while `Action.DoAction` answered `true`. Building the half that
performs the click was this round's, and 731's reading that it needs **no new message** held — the
boundary gained nothing and no variant changed shape, which is five rounds running.

## What the clauses say

- **§12.7.5.2.1's Table 229 bit 15** is the rule for a click on a button that is already on: "If
  set, exactly one radio button shall be selected at all times; selecting the currently selected
  button has no effect. If clear, clicking the selected button deselects it, leaving no button
  selected." It is **"(Radio buttons only)"** in the table's own first three words, so a check box's
  flags cannot reach it.
- **§12.7.5.2.3** names what a click *sends*: `/V` is "a name object representing the check box's
  appearance state", the names are the file's own invention, and the off state "shall be stored in
  the appearance dictionary under the name `Off`". A widget whose `/AP` names no on state has no
  name to send and is reported rather than guessed at.
- **Table 227 bit 1** is checked before either, in every host and in the core — "the field shall not
  be modified by the user".
- **§12.7.5.2.4's first sentence is about a *widget* and was being disobeyed**: "Like check boxes,
  individual radio buttons have two states, on and off", with §12.7.5.2.3 making the exclusion a
  `shall` where Table 229 bit 26 is clear — "at most one radio button in a field shall be set at a
  time".

## What is new

- `viewer_host::form::Clicked`, `::toggling` and `::clicked` — §12.7.5.2's rule as one closed enum,
  two doors into it, and one walk from a point to a widget. Matched **exhaustively** in five places.
- `viewer_host::geometry::covers`, which `viewer-ui` had privately and `clicked` wanted second.
- `viewer_core::this_widgets_control`, so §14.7.5.3's per-annotation map answers for the annotation.
- `viewer-gtk`'s `write_back` follows §12.7.5.2's toggles, and a refused click puts the button back.
- `viewer-qt`'s `Placement` carries §14.9.3's two names and Table 227 bit 1 where it carried one
  name and neither flag.
- Three tests that fail against the defect they are about, run that way before being believed
  (trap 13).

## What was measured, and on what

`annotation-button-widget.pdf` on a real AT-SPI bus under `Xvfb` — a `dbus-python` client walking
from the registry root, `DoAction(0)` on each of the nine nodes declaring `click`, and `GetState`
read back **after each one**.

- **Six of nine clicks give a value and three are refused on Table 227, identically in
  `pdf-viewer`, `pdf-viewer-gtk` and `pdf-viewer-qt`** — line for line in each host's
  `--trace=access` log. Before this round the two native windows gave a value to none of the nine.
- Both native windows were photographed either side of the walk: every one of the nine controls
  agrees with the field it is over, and a person's mouse click on one radio button of a set visibly
  unchecks its sibling.
- **The briefing's "nine of nine" is not obtainable and the standard is why**: three of the nine
  belong to fields the document marks read-only, and one more is Table 229 bit 15 on the one button
  of a set already on. Parity here means all three windows refusing the same four for the same
  printed reasons.

## Three things that were wrong, and how each was found

- **§12.7.5.2's rule was written three times and the three had stopped agreeing.** Only `viewer-ui`
  asked Table 227 bit 1 before sending an edit; the other two relied on a disabled control, which is
  a fact about a *person's* click and not about the two other ways one arrives. Found by reading the
  three before writing a fourth.
- **A radio set announced every one of its buttons as selected.** `referenced_objects` keyed
  §14.7.5.3's map by the annotation and stored the *field's* control under each. Found on the bus:
  clicking one node's `DoAction` moved its sibling's `checked` too. `pdf_model::form::Widget::on`
  was the right fact and had been beside the wrong one all along, its doc comment saying which is
  which.
- **`viewer-gtk` never wrote a toggle back where `viewer-qt` always did**, since ADR 0244. Trap 1 in
  its purest form: with the write-back removed and everything else in place, the bus still reports
  six of nine toggling and the window's pixels do not move at all.

## Gates

The full §2 sequence, this being a fifth round. §4's sweeps. §5's binaries installed from this
worktree's own build directory, twice — once before the first measurement and once after the
`viewer-core` fix, because a stale binary is a measurement of the past.

`doc/conformance/ledger.toml` §12.7.5.2, §12.7.5.2.3, §12.7.5.2.4, §14.7 and §14.7.5.3 all carry
what this round did.

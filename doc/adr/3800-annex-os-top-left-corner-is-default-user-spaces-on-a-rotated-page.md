# 3800 — Annex O's top left corner is default user space's, on a rotated page too

Session 2274, review 2271's unfiled note (lines 84–90). Status: **accepted**. Context: ADR 0209,
ADR 0357; ISO 32000-2 Annex O §O.2.2 Table O.4, §8.3.2.3, Table 31, §12.3.2.2.

## Context

`zoom`, `viewrect` and `highlight` measure from "the top left corner of the page", and
`viewer_core::Open::in_default_user_space` takes that corner on the unrotated display box. On a page
with `/Rotate` the corner a person sees at the top left is another one. Review 2271 left it as a
reading to record.

## Decision

The code is right, and on a rotated page the corner is default user space's: least x, greatest y.

- §O.2.2: "All coordinate values ( left , right , top , and bottom ) shall be expressed in the default
  user space coordinate system." Table 31's `/Rotate` is "[t]he number of degrees by which the page
  shall be rotated clockwise when displayed or printed": it turns the output, not the coordinate
  system; §8.3.2.3's "subject to alteration by the Rotate entry" is said of where the axes point on
  the medium, and the coordinates of a point do not change with it.
- A corner of the turned page would put the values in a system whose axes are not default user
  space's, which is the one thing the rule forbids; measured from default user space's own top
  (greatest y) downward, only the origin and the direction of y differ, which the rows themselves
  state.
- `view` in the same table takes §12.3.2.2's positions, which are default user space's and are not
  turned; reading `zoom`'s left and top from another corner would make two rows of one table name
  different points by the same numbers.

## Consequences

- `crates/viewer-core/tests/fragments.rs` holds a `/Rotate 90` page whose highlight measured this way
  shows at the turned page's top right, derived from the rotation.
- No host reads the corner itself: every host takes the quadrilateral `Query::Highlight` answers and
  the view `viewer_core` sets, so the reading is in one place.

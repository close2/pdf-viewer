# 3792 — The last nine held sites are read, and `HELD` is deleted

Session 2272, the silent-repair campaign's census, last pass. Status: **accepted**. Context: ADR
3477, ADR 3521, ADR 3574, ADR 3777, ADR 0255; ISO 32000-2 §8.7.3.1 Table 74, §8.10.2 Table 93,
§11.4.7, §11.6.5.1, §12.5.5.

## Context

ADR 3777 left nine sites held in seven files of `pdf-model/src/content`, two other rounds' files,
and said that when they fell `HELD` should be deleted and the census should forbid any silent site.

## Decision

Each was read against its enclosing function's documentation (ADR 3521's rule):

- **A repair, now logged** — a new class `model/tiling-pattern-entry-unusable` (§8.7.3.1), mapped to
  section 5.1 alone: Table 74 makes a tiling pattern's `/BBox` and `/Resources` "(Required)", and one
  that is not four finite numbers was tiled from a cell of the step's size at the origin, unclipped,
  and one absent or of another type read as defining nothing. Logged at the content that painted
  with the pattern (`pattern/tiling.rs`); the two uses of the absent box (`pattern.rs`,
  `pattern/lattice.rs`) say where it is logged.
- **A repair the comment claimed and nobody made** — a soft mask's group's `/Resources` of another
  type: `soft_mask.rs` said it was "logged where the group is run", and the run did not log it. It
  now logs `model/form-resources-not-a-dictionary`, as a form's does, the group being §11.6.5.1's
  form `XObject`.
- **Not repairs, marked**: the two defaults of `transparency.rs` are §11.4.7's page group and §12.5.5's
  own "non-isolated, non-knockout transparency group"; `into_parent.rs`'s two are digits of this
  reader's own grid index; `marked.rs`'s is a colour this reader computed, whose marker stood one
  attribute too far above it.
- **`HELD` is deleted.** The census now fails naming every site, by file and line, that has no class
  within its window and no marker above it.

## Consequences

- The census holds no table a round could add a line to. A marker written without a reading is still
  possible and is reviewed, not counted.
- Left: `transparency_group` reads a `/Group` that is no dictionary, or one whose required `/S` is
  absent, as no group at all, through `?` — a shape the census does not see. It is a repair to log in
  a later pass.

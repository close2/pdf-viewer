# 1073 — A table cell's rectangle is its row's band, not its row's maximum

Session 1059. Status: **accepted**. Settles how §14.8.5.4.5's last unimplemented sentence is applied
to rectangles this program *measured* rather than laid out, so that a later round reading the clause
beside the code does not read the difference as a defect.

Context: `crates/pdf-model/src/structure.rs` (`table_cell_rectangles`,
`BlockProgression::stacks_vertically`, `TableStack::innermost_table`),
`crates/viewer-core/src/accessibility.rs` (`TableCell`, `equalise_table_cells`, `places`),
`crates/viewer-core/tests/accessibility_census.rs`, ADRs 0301, 0486, 0768;
`doc/traps/instruments-and-reports.md` trap 13.

## 1. The sentence, and why it is not arithmetic anybody can copy

ISO 32000-2 §14.8.5.4.5 derives a `TH` or `TD`'s content rectangle from the marks in the cell and
then makes it the grid's:

> The cell's height shall be adjusted to equal the maximum height of any cell in its row; its width
> shall be adjusted to the maximum width of any cell in its column.

It is a `shall`, it is the last of the clause's four steps for a cell, and it is derivable here —
`TableStack` already fills the grid as `viewer-core` walks the structure tree, because §14.8.5.7's
assumed `/Scope` needed it.

What is not transferable is the word *maximum*. The clause is addressed to a process that is
**placing** cells: a row's cells are set against one before edge, so giving each of them the largest
height in the row fills the row exactly. This program places nothing. Each cell's rectangle is the
bounding box of what that cell actually drew, and two cells of one row rarely begin at the same
edge — a cell whose text sits a little lower starts lower. Extending each cell from *its own* before
edge by the row's largest extent therefore pushes the short cells past the bottom of the row they
are in, and hands a magnifier a rectangle that overlaps the row below.

## 2. The decision

**The operand is the row's band**: the interval the row's cells occupy between them, along the axis
§14.8.3.3's writing mode calls block progression. A column's band is the same along the inline axis,
and a cell's rectangle is the intersection of the two.

- Where the producer's own layout did put a row's before edges together — which is what a table
  looks like — the band **is** the clause's maximum, and the two readings agree exactly.
- Where it did not, the band is a bound rather than an overhang. That is the direction every other
  rectangle in this family already errs in: §14.8.3.3's row records that a container's width is
  taken as the union because the standard leaves it to a reference area this program does not lay
  out, "a bound, never an underestimate".

Three consequences are part of the decision rather than details of it:

- **A cell that spans rows joins no row's band** and is then given the union of the bands of every
  row it covers. Table 384's `/RowSpan` makes such a cell as tall as the rows it occupies, so
  letting it into either row's band would make every cell beside it that tall.
- **A cell that drew nothing is placed by its row's band and its column's.** Every number in that
  rectangle came off a neighbour's marks and the grid; it is the clause's own construction — the
  cell's height *is* its row's and its width *is* its column's — and it is where the empty cell of a
  table is on the page. 106 cells of the corpus are placed this way and had no place at all before.
- **`/Width` and `/Height` stay unread**, and the reason is the order of the clause's sentences
  rather than ADR 0301 alone: the override feeds the *implied* size, and the row and column
  adjustment comes after it, so a cell's final extent is its row's and its column's whatever the two
  attributes said. Those maxima are taken here from what was measured, which outranks what was
  stated by ADR 0301's argument as ADR 0486 amended it.

## 3. What was rejected

**The literal arithmetic** — each cell's own before edge plus the row's largest extent — for the
overhang above. It is the clause's wording applied to an operand the clause does not have.

**Leaving the sentence unimplemented**, which is what the ledger row said for 1058 sessions: that a
cell's rectangle is "its own marks' and not its row's". A reader of a table is pointed at cells, and
the ink in a cell is not the cell; the clause says so in one sentence and nothing was executing it.

## 4. How it is measured

`viewer-core --test accessibility_census` asks `viewer_core::places` the same question twice — once
with `AccessibilityNode::cell` as it is and once with the grid withheld — and prints how many cells
the adjustment moved and how many it placed outright. Asking the same function twice rather than
recomposing the precedence is the rule the three counts above it already follow, and the withheld
arm is trap 13's control: an adjustment that did nothing would print zero rather than a tick.

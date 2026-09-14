# 1059 — A cell is its row's band, and the two attributes that vanish into it
§14.8 had nine `partial` rows. Two are closed and a parent note named two discharged debts.

## §14.8.5.4.5 → `implemented`
The last thing this row owed was the table cell's `shall`: "[t]he cell's height shall be adjusted to
equal the maximum height of any cell in its row; its width shall be adjusted to the maximum width of
any cell in its column." Derivable since §14.8.5.7's `/Scope` made `TableStack` fill the grid.
`structure::table_cell_rectangles` is the arithmetic, `BlockProgression::stacks_vertically` says
which edges a *height* lies between (Table 379 measures it "in the block-progression direction", so
a `TbRl` table's rows equalise in x), `TableStack::innermost_table` is the identity `CellPlacement`
lacks, and `viewer_core::places` applies it through a new `AccessibilityNode::cell` that crosses the
sandbox — a host derives none of the three: the column is a fact about the grid, the spans are in
attribute objects, the mode is inherited.

**The operand is the row's *band*, not its maximum, and that is ADR 1073 rather than the clause's
wording.** The clause can say maximum because a layout process sets a row's cells against one before
edge; this program measures what each cell drew, so each cell's own edge plus the largest extent
would push the short cells out of their row. The band equals the maximum where the producer's layout
did put the edges together and is a bound where it did not. A spanning cell joins no band and takes
every band it covers; a cell that drew nothing takes its row's band by its column's.

**`/Width` and `/Height` are disposed rather than owed**, on the order of the clause's
sentences: the override feeds the *implied* size and the row and column adjustment comes after it,
so a cell's final extent is its row's and its column's whatever the attributes said — and those
maxima are measured here, which outranks stated by ADR 0301 as ADR 0486 amended it.

## §14.8.5.4 → `implemented`, §14.8's note corrected, and four rows read and left
The aggregate's seven children are all `implemented` or `inapplicable` now; its note said the family
owed `/Width` and `/Height`, answered twice. §14.8's own note named three debts there of which two
were discharged (§14.8.5.8's `/Type` and `/Subtype`, §14.8.5.4.5's derived rectangles), leaving
§14.8.5.3's `NSO` owner alone. That row's remainder needs a rank as well as a membership and is
paired with §14.7.4.2; §14.8.2.2.1 and §14.8.2.2.2 both hang on "[a]ny content that is not included
in the structure tree is an artifact", needing a consumer of artifact-by-absence that does not
exist; §14.8.2.3's is the rejoining decision, a consumer's too. None was rewritten.

## Instruments
`accessibility_census` gains one printed line (not ratcheted, `doc/todo/05`): **30 576 cells placed
in a grid, 28 157 moved, 106 placed by nothing else** — elements with no place fall 1 989 →
1 883. Measured by asking `places` the same question with the grid withheld — trap 13's control, and
why `contained` was corrected: it had begun counting the new route as the enclosure one.

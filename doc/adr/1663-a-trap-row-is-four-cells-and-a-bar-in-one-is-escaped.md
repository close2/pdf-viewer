# 1663 — A trap row is four cells, and a bar inside one is written `\|`

Session 1413. Status: **accepted** and **built**. Narrows ADR 1036's index format; supersedes nothing.
Code and test: `tools/conformance/tests/traps.rs` (`cells`, `row_shaped`, `malformed_rows`).

**The defect.** `doc/traps/README.md` is a table of four columns, and a cell quoting a shell pipe
(`… | tail`) ends at the bar under GitHub's table syntax, so the row renders with five columns. The
reader split every line on `|` and kept only four-cell rows, so such a row was dropped, and the only
failure left was its group file's heading "with no row naming that group" — beside a row that is
plainly there. The last merge met exactly that on trap 127's row.

**Decision.** A bar inside a cell is written `\|`, which GitHub reads as a character of the cell,
in a code span or not. The reader splits at unescaped bars only, and a line shaped like a row — a
table line whose first cell opens with a digit — that is not four cells fails by its line and its
trap, with the spelling that fixes it; the heading's own complaint is then not repeated.

**Calibrated** (trap 13): a bare `| tail` planted in trap 127's row failed the gate with
`trap 127: its row (doc/traps/README.md line 176) has 5 cells, not 4 — a `|` inside a cell ends the
cell; write it `\|``, and the reversed plant passed; the reader's own test holds an escaped bar, in
a code span and outside one, as a cell's character.

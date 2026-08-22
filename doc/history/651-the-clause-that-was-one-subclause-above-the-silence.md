# 651 — The clause that was one subclause above the silence

One contradicted group, taken apart. Parallel round, worktree `r651`, branch `round-651`.
**No pixel moves**; what changed is a comment that said the standard decides nothing, a ledger row
that contradicted its own neighbour, and a group note whose every sentence about a reference was
false. ADR 0480 has the argument and the tables.

## Which group, and why

Fourteen contradicted groups are non-empty. `git blame` over the run of group comments says that
thirteen of them have been re-opened since they were written and one has not:
`CONTRADICTED_NEGATIVE_LINE_WIDTH`, written whole in `244fd96c` and untouched since. Its note is also
the only one in the file that says what a reference *draws* with no number behind it — "a very faint
one, consistent with the magnitude", "`ghostscript` draws something between the two" — where every
other group states ink, a bounding box or a closed form.

## What the page is

`issue19633.pdf` is 312 652 bytes of iTextSharp form with an `/AcroForm`, three document-level
JavaScript actions and seven form XObjects, not the "one operator" the note described. Its crop box
`[131.5 439.89 383.0 600.89]` is what makes the note nearly true: ten of the eleven `Do` invocations
place `/Tx BMC EMC` stubs above page y 771, outside it. Inside it there is one stroked diagonal,
171.51 pt long at 22.56°, asked for at `-0.1 w` under a `cm` scale of 0.85409 — a device width of
0.0854.

## Measured, not looked at

Ink ÷ mark length is the width a renderer actually painted:

| | ink, whole pixels | ÷ 171.51 |
|---|---|---|
| ours | 172.54 | **1.006** |
| `hayro` | 170.87 | 0.996 |
| `ghostscript` | 96.81 | 0.564 |
| `poppler` | 42.44 | 0.247 |
| `mupdf` | 37.49 | 0.219 |

The document asked for 0.0854. Nobody drew the magnitude "0.1 of a pixel's coverage" the note
attributed to two of them.

ADR 0419's ladder — one rule at seventeen widths on a 200 × 200 page — is the right instrument and
had never been run below zero. Continued through zero, its positive half reproduces that ADR's
72 dpi table to the digit and its new half says: **`poppler` and `ghostscript` stroke the
magnitude** at every width and (swept over nine angles) every angle; **`mupdf` paints nothing within
5° of an axis and its own 0.2-pixel floor beyond 10°**, two answers to one question; **ours and
`hayro` paint one device pixel throughout**. So the pair that outvotes us on this page is one
renderer reading the magnitude and one showing a floor, coinciding inside the fixed tolerance — at
`-1 w` they are 1.02 and 0.00 apart, the ladder's widest disagreement.

## The clause

`spec-errata emit` prints one annotation under the heading *8.4.3.2 Line width*, and it is not about
line width: Issue #368's caret sits at `/Rect [358.768 239.123 367.781 246.468]` on page 175, which
`pdftotext -bbox` places between §8.4.2's last line and the §8.4.3 heading. Errata Collection 3
touches §8.4.3.2 nowhere — the fifth time this month a round has had to read a caret's rectangle
rather than its heading.

§8.4.3.2 gives the range and stops. **§8.4.1 decides a value outside one, and names this parameter
while doing it**: numeric values "shall be clipped into valid range, if necessary", a device
adjustment belongs to the painting operator, and the adjusted value "shall not be stored back into
the graphics state". All three are what this tree does. The magnitude reading is the one the first
sentence forbids.

**The sentence was already quoted in the same crate as the code that said it did not exist** —
`content.rs`'s `miter_limit`, for the parameter §8.4.1's own list names one *after* the line width —
and the ledger's §8.4.1 row had stated the line-width half correctly while §8.4.3.2's row, two rows
down, called the same clamp a documented choice among three readings.

## Changed

- `content/run.rs` (`w`), `content/ext_gstate.rs` (`/LW`), `pdf_render::Stroke::device_width` — the
  clamp is documented as §8.4.1's requirement rather than as a choice, with the clause quoted; the
  old inline quote of §8.4.3.2 was inexact ("non-negative" for `doc/md`'s "nonnegative") and is now a
  verbatim blockquote.
- `line_parameters.rs::a_negative_line_width_is_clipped_into_range` — new, asserting the clipped
  value by both of §8.4.1 NOTE 1's routes.
- `oracle.rs` — the group note rewritten around the ladder and the clause. **The name is kept**, and
  says so: the page really is about the negative line width.
- Ledger §8.4.1 and §8.4.3.2 — the two rows now agree, and both cite the new test.
- `doc/traps/pixels-and-rasterisers.md`, `doc/oracle-and-corpus.md` — the tally sentence. This round
  does **not** make it thirteen for thirteen: it is the first examination where a group's *name*
  survived, and both files now say the note is the thing to distrust.

## Owed

- `mupdf`'s two answers, `poppler`'s and `ghostscript`'s magnitude, and `hayro`'s one-pixel floor are
  all unreported upstream; `doc/HAYRO_ISSUES.md` does not name the last.
- Ours at 0.001 of a device pixel reads 0.0079 where ADR 0419's table says 0. That is the direction
  that ADR argued for, and no session claimed it.

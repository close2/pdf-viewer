# ADR 0593 — The fifth fill, and a count that was four in four places over five arms

Status: accepted, 2026-08-24. Session the seven-hundred-and-sixteenth, a clause round under
`doc/todo/01`, reading one family's `partial` rows against each other and against the code — ADR
0538's method in its sixth round (0551, 0560, 0567, 0579, this). Amends §12.5.6.6, §12.5.6.7 and
§12.5.6.8 in the ledger; changes one expression and three doc comments in
`crates/pdf-model/src/appearance.rs`; renames and widens one test in
`crates/pdf-model/tests/annotations.rs`; adds one section to `doc/errata-read.md` and one marked
correction to ADR 0192. **No status moves and no pixel moves.** Extends ADRs 0192, 0329 and 0567.

## 1. The family, and why the head is not always the family to read

ADR 0567's search was run on this base rather than read off a document, with 0579's two rules
applied: strip the clause-level parents, and let the total rank the family while the pairs choose
the reading. **§12.5 heads it again**, with §12.8 second and §12.7 third.

**That head is partly the previous round's own writing, and the ranking cannot say so.** Measured
on both sides with one instrument — the ledger as it stood before session 710's commit, and as it
stands now — §12.5.2 ~ §12.5.5 goes from 17 shared rare sequences to 21 and the family's total from
221 to 225. 710 read exactly that pair and rewrote both rows in one voice, which is what a round
reading a family does, and the score it leaves behind is higher than the one it found. So:

> **A family the last round read scores higher for having been read.** The ranking is
> self-reinforcing over one round, and the pair to take is the strongest one the previous round
> *named and did not read*, not the one at the top of the family's list.

Here that is §12.5.4 ~ §12.5.6.8, at 24 — the strongest pair below any clause-level parent in the
whole ledger bar two — which ADR 0579 §1 named and left. It is recorded in `doc/todo/01` as a third
rule for reading the output.

## 2. What the pair said, and what reading it opened

The two rows share a quotation of §12.5.4 — "[s]uch dictionaries may also be used to specify the
width and dash pattern for the lines drawn by line, square, circle, and ink annotations" — and a
corpus count. Both are right, and enumerating the `/BS`-bearing tables confirms the division they
rest on: Tables 176, 177 and 191 give a link, a free text annotation and a widget a `/BS` that is
"the annotation's border", and Tables 178, 180, 181 and 185 give a line, a square or circle, a
polygon or polyline and an ink annotation one that supplies width and dash alone. `Border::simulated`
is asked by exactly the first three.

So the pair was clean, and what it did was put this round on the tables the two rows argue from.
`spec-errata emit` over the pages Table 179 spans is where the round's findings are, which is the
briefing's rule doing its work: **a pair that survives its reading has still chosen the pages.**

## 3. The finding: Table 179 fills five, and four places said four

`Ending::filled` returns true for `Square`, `Circle`, `Diamond`, `ClosedArrow` and
`ReverseClosedArrow`. The published Table 179 names the fill on the first four and says of
`RClosedArrow` only that it is `ClosedArrow`'s arrowhead "in the reverse direction from" it, so the
fifth was **a reading** — a reversed shape is the same shape — taken in ADR 0192 and never written
down as one.

**Errata Collection 3 Issue #515 states it.** A `Caret` whose `/Contents` is "filled with the
annotation's interior colour, if any", closing the sentence with a full stop, sits two points past
the last word of `RClosedArrow`'s second
line; `doc/errata-read.md` has the arithmetic that places it. `/State` is `Review`/`Completed`.

**The reading was right and the prose around it was not.** Four places said *four*:

| said four | over |
|---|---|
| `Ending::filled`'s doc comment | a `matches!` with five arms |
| the test's doc comment | a loop of five names |
| the test's *name*, `only_the_four_endings_…` | the same loop |
| §12.5.6.6's ledger row, "the four styles the table fills" | the same table |

Each is now five, and the erratum is what makes the fifth the table's sentence rather than this
crate's inference. **Nothing drawn moves.** This is the shape 710 named running in its most
comfortable direction: a count copied from a row into a comment into a test's name, none of which
was ever re-derived, and an erratum three years younger than any of them.

**`check` could not have printed it.** Issue #515 is a `Caret` with no `StrikeOut`, so there is no
struck text for a quotation to match — the first of the three ways that command is blind, and the
first to be met since the third was named.

## 4. The second finding: the fill was decided twice, and the test could not see it

Calibrating the renamed test per trap 13 — remove `ReverseClosedArrow` from `filled` and check the
run fails — **the run passed.** `draw_ending` asks `Ending::filled` once, for the three shapes that
are a square, a circle or a diamond, and the arrowhead arm three matches below decided for itself:

```rust
stream.paint(closed && interior != Colour::None, true);
```

`closed` is `ClosedArrow | ReverseClosedArrow`, so the two expressions agree on all ten names and
have always agreed. What the duplication cost is not a pixel but the *reach of a correction*: the
one function that states which of Table 179's shapes the standard fills governed three of the five
it names, and the two the erratum is about were outside it. A test guarding `filled` therefore
guarded three arms and asserted five.

The arm asks `fill` now — provably the same value, since `filled` is false for both open arrowheads
— and the calibration then fails on `/RClosedArrow` by name. **This is 701's shape inside one
function**: a claim held in duplicate has somewhere to disagree with itself, and the place it had
not yet disagreed was the place a correction would have had to arrive.

## 5. The third finding: the standard says our instrument is short a word

Issue #513, on the same page, is an EDITOR NOTE rather than a change:

> The row height in the ISO PDF file obscures the end of the sentence. The text is unchanged but
> noted here for clarity.

`doc/md/` carries exactly that damage: Table 179's `OpenArrow` cell ends at "an open" and
`ClosedArrow`'s begins with the word that finishes it. `--bin quotations` has been printing ADR
0192's copy of the sentence as a diverging document span at the head of its own output, and the
answer is the sweep's own instruction — *suspect the conversion before the document* — supplied for
once by the specification. The quotation is correct and stays; the ADR now says why the sweep prints
it.

**A hit a sweep prints every round is not a hit nobody has explained**, and the difference is worth
a sentence in the document rather than a fix in the quotation.

## 6. What was deliberately not done

- **`doc/md/` is not patched.** It is a conversion of a document this project does not own, checked
  in as the instrument the conformance gate reads; hand-editing it would make the gate's agreement
  with the standard a property of our edits. The divergence is recorded where a reader meets it —
  in the ADR that quotes the sentence and in `doc/errata-read.md`.
- **Issue #524 changes nothing and is recorded anyway.** It strikes `rectangle` and writes `array`
  in the type column of `/RD`, in Tables 177, 180 and 187. Nothing in this tree calls `/RD` a
  rectangle and `appearance::differences` reads four numbers in the clause's order, so establishing
  the absence is the whole of the work — and a one-word strike is under `check`'s four-word floor,
  which is why `emit` is what found it.
- **No report is added.** Every requirement these three errata touch is executed; there is nothing a
  reader is owed a sentence about.

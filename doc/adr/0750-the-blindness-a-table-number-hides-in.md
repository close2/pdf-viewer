# 0750 — The blindness a table number hides in, and the repair that was not owed

Status: accepted.
Context: ADR 0746's second finding — Issue #700 renumbers Annex O's two tables where no instrument
in this tree can see it — carried forward with two jobs: repair what stands on the retired numbers,
and close the blindness or say why it cannot be closed.

**0749 is a sibling round's.** This number was taken one above the tip on that reservation.

## The first job was not owed, and the tree is what says so

The round was set to *move the lines standing on the retired numbers to the current ones*. **The
tree contradicts that instruction and the tree wins.** The standing answer to a number an erratum
has moved is stated in three places, and each states the same three parts:

- `doc/errata-read.md`'s sixth blindness — "the published numbers stay, the amendment is recorded
  in the row, and one command prints which numbers stand on moved ground and how much of the tree
  stands with them";
- the three Annex O ledger rows, which record the amendment and say the numbers are not renamed;
- **`spec-errata moved`'s own closing sentence**, which the program prints on every run: the
  numbers are not changed anywhere, because `doc/md/` is the published text and the citation gate
  resolves against it.

The reason is not ceremony. `doc/md/ISO_32000-2_sponsored_EC3.md` captions those two tables
`Table Annex O.3 -PDF object identifiers` and `Table Annex O.4 -PDF open parameters`; the standard
this project reads has no `Table Annex O.1` at all. A tree citing the amended designation would be
citing a caption no reader can find, and would have swapped a number that resolves for one that
does not in order to be right about a document nobody here holds. Errata Collection 3 is *evidence
about the standard* (ADR 0601), not a corrected copy of it.

**And the citations were checked rather than assumed correct.** Every place naming `Table Annex
O.3` attributes one of `page`, `nameddest`, `structelem`, `comment` or `ef` to it and every place
naming `Table Annex O.4` attributes one of `zoom`, `view`, `viewrect`, `highlight`, `search` or
`fdf`, which is exactly how the two captions divide the eleven parameters. The single line naming
both is `fragment.rs`'s module comment dividing them in one sentence. There was no repair to make,
which is the answer a round has to be able to return.

What *was* owed is the sentence three ledger rows carried — **no instrument in this tree can see
it** — which this round makes false and therefore had to correct. That is the ordinary decay a
claim about this tree takes, and `doc/todo/01`'s fourth sweep is the instrument for it.

## The second job: the predicate, built

ADR 0746 wrote the predicate down rather than building it: *a `StrikeOut` whose covered text
matches a table designation, paired with a `Caret` on the same page whose contents match another*
— no verb, because a bare pair carries none, and no clause number, because a table is not one.
`spec-errata renumbered` is that command. Two things about it differ from the written shape, and
both were found by running it.

### The pairing is the `/IRT` group, not the page

§12.5.6.2 makes a strikeout and the caret replacing it two marks of **one change**, joined by
Table 172's `/IRT`, and `Note::change` already carries that join for `applied`'s sake. Pairing on
the page instead would join two renumberings that share one — which is not hypothetical: Issue
#124 puts four strike-and-caret pairs on page 483 of ISO 32000-2 alone.

### The shape alone is nine parts noise, and the second grounding is the finding

The written predicate says *matches a table designation*. Grounded the only way that can be
checked — the conversion of that same document **captions** a table with it, so that a version
number or a clause number cannot qualify — it admits **eleven** annotations of Errata Collection 3,
of which **nine are integers struck in body text**:

| issue | what it really is |
|---|---|
| #124, four pairs on p.483 | array indices corrected to be zero-based, `1`→`0`, `3`→`2`, `2`→`1`, `4`→`3` |
| #133, two pairs on pp.779–780 | two NOTEs renumbered, `2`→`1` and `3`→`4` |
| #144, p.336 | `104`→`98` inside a Type 3 font example |
| #446, p.145 | `4`→`0` in a Type 4 function's domain |
| #527, p.51 | `7`→`9` in an LZW encoding example |

A bare `3` is a table designation **and** an array index **and** a NOTE's number, and nothing in
the annotation says which. So there is a second grounding, and it is what makes the report worth
reading: **does the clause the annotation is filed under caption that very table?** A strike over a
caption is inside the clause that owns the caption; a strike over an array index five hundred pages
away is not. On this collection it separates Issue #700's two annotations from all nine of the
others, with nothing in between.

**It ranks rather than filters.** ADR 0712's placement rule — established over six consecutive uses
of the errata rule — is that §12.3.3's outline files an annotation one clause away from its subject
often enough to be written down every time; a filter resting on it would turn a placement artefact
into a renumbering nobody ever sees, which is the blindness this closes. So the far rung is printed
and counted, one line apiece with the section it was filed under, and the near rung carries the
ground.

### The ground is a designation rather than a SECTION SIGN

`moved` counts what stands on a clause number, which `conformance::citation`'s SECTION SIGN scanner
already finds. A table is cited by name — `Table Annex O.3's` — and **`conformance::citation` could
not see that at all**: `read_tables` parses the digits after `Table ` into a `u16` and stops, so
`Table Annex O.3`, `Table D.2` and `Table 125a` were read as no reference. `Scan::designations` is
the wider population, parsed from the same scan, with the numbered one left exactly as it was
because it is what the conformance gate checks.

**A designation is a token after `Table ` that carries a digit**, optionally behind the `Annex `
the caption itself prints. That one rule excludes the whole of this tree's prose about tables in
general — `Table N`, `Table NNN`, `Table structure`, `Table or`, `Table numbers` — and `caption_of`
reads the same shape off `doc/md/`'s caption lines, so the two sides of every comparison here are
one rule asked twice.

## What the run says, and it is a number rather than a claim

At this round's base commit, the two retired designations carry **27 source citations and 16 places
in this project's documents and the ledger** for `Annex O.3`, and **34 and 14** for `Annex O.4` —
91 places between them, counted by the instrument rather than by a grep, which is the difference
between a line and a citation. ADR 0746's figure was 75 lines across 27 files from a grep at its own
base, and both are re-derivations rather than quotations.

## The blindness that is left, named rather than closed

`--bin tables` and the conformance gate both take a table number as a `u16`, so **no instrument in
this tree checks a non-numeric table designation for correctness** — `Table Annex O.3` and
`Table D.2` are not verified to exist any more than `Table 106` was before the thirteenth session.
`Scan::designations` is the population a gate for that would stand on, and every such designation
this tree cites resolves in ISO 32000-2 today except one: `Table A.19`, cited twice and attributed
in both places to **ISO/IEC 15444-1**. So the gate would need the foreign-standard rule
`read_citations` already has for a SECTION SIGN before it could be a gate at all, and inventing that
rule was not this round's subject. It is written into `doc/todo/01` beside the sweep rather than
left as a thing somebody notices again.

## Calibration

Trap 13, both directions, run above the commit that makes the change, by planting two comment lines
in `crates/viewer-core/src/open.rs` and restoring the file from a copy afterwards:

| planted | the run says |
|---|---|
| `Table Annex O.4`, a designation Issue #700 retires | `Annex O.4`'s source citations rise from 37 to 38 and `crates/viewer-core/src/open.rs:1868` is named |
| `Table 149`, a designation no erratum in the collection moves | absent from the report entirely, and `Annex O.3`'s count does not move |

The instrument's own discriminator is calibrated a second way, in
`renumbered::tests::a_strike_inside_the_clause_that_captions_the_table_is_the_closer_rung`, on the
two shapes the first run produced: a strike inside the clause that captions the table is the near
rung, and an array index in a clause that captions nothing is the far one *even though the document
captions a table of that number*, which is exactly why the caption alone cannot decide it.

## Consequences

- `spec-errata renumbered` exists, and the sixth blindness `doc/errata-read.md` names is closed. It
  is not a gate, for `doc/todo/48`'s reason: it parses fourteen PDFs.
- `conformance::citation::Scan` carries every table designation the tree cites, not only the ones a
  `u16` can hold. The gate's own population is untouched, and its test says so.
- `conformance::clause::caption_of` is the caption rule in one place, with
  `ClauseIndex::table_title` reading through it and `designated_table_title` and `captions_table`
  beside it.
- The three Annex O ledger rows no longer claim that nothing here can see the amendment.
- Not one citation moved, and the reason is written down where the next round asking will meet it.

# 829 — The base-85 bound a lost superscript hid, and step 6's first run

**Finding:** ISO 32000-2 §7.4.3's first "shall never occur" condition prints as *greater than
232 - 1* — the exponent lost in the ISO PDF's text layer and in `doc/md/` alike, so no quotation
gate in this tree could ever have seen it — and this filter had decoded every base-85 group above
the missing bound as four `0xFF` bytes in silence, under a ledger row that claimed all three of the
clause's conditions were refused.

Date: 2026-08-29.
Argued in: [ADR 0757](../adr/0757-a-bound-the-printed-page-could-not-state.md).
Branch: `round-829`, from `0b6709f7`. Not merged.

Touched: `crates/pdf-syntax/src/filter.rs`, `doc/conformance/ledger.toml` (§7.4.3),
`doc/errata-read.md`, `doc/todo/01-ledger-partial-rows.md`, `doc/adr/0757-…`, this file.

## The round

The errata selection rule's **nineteenth** use, and the first run under `doc/todo/01`'s step 6.

All three rankings flat for a third consecutive use: over live rows 5 rows tie at two annotations
with 3 at one, over every row 25 tie at two with 17 at one, and by issue 24 tie at two with 19 at
one — the eighteenth use's field less its four verdicts. The base count reproduced the closing
arithmetic for the **tenth** consecutive use: 302 issues carry a strike or a caret under the
single-issue line parse and 50 are named nowhere, with the multi-issue parse's 310 and 52
reproducing over the same eight second-position numbers. The distribution is 35 settled-only, 8
live-touching and 7 on no row at all, a plain subtraction from 37, 10 and 7.

**Step 6 changed the head**, which was the round's methodological question, and the demonstration is
exact: the field offers exactly one requirement-level substitution, Issue #58, and it is on an
`out-of-scope` row under the clause-13 exclusion. Ranked together, the tie-break takes it. Counted
apart, the tier is empty and the head falls to the next.

The step also showed what it still owes. Its column is keyed on the row `emit` files an annotation
under, and ADR 0712's placement rule files by the outline section of the *page* — so the head step 6
then produced, Issue #679, turned out to be Table 223's, which belongs to §12.6.4.18 and carries the
same exclusion. One clause-13 head was exchanged for another, and reading it rather than counting it
is what caught the second. The recipe now carries the caution.

Six issues read to a verdict: Issue #58 and Issue #679 confirm inside the exclusion, Issue #280
confirms on Table 34's `/ColorSpace` cell where `Colour::by_name` already answered every bare name
the erratum adds, Issue #306 and Issue #80 confirm on two tables that describe rather than require,
and **Issue #98 pays** — the payment arriving at the fourth issue of the walk downward, which is
step 4's practice for the fifth use running.

Both routes of the ASCII85 decoder now refuse the two conditions that were silent, each in the voice
ADR 0343 gives it, and §7.4.3's row no longer claims to enforce a bullet it never named. Four trap-13
calibrations, in both directions for each of the two conditions, above the commit that makes the
change.

`doc/errata-read.md`'s blindness index, added under load by the eighteenth use, was checked: the two
lists and their six-and-eight contents are right and the sixth's closure by `spec-errata renumbered`
is right, while the illustrative sequence of ordinals was neither what the file says nor complete —
it dropped *fourth*, which is the ordinal the tables use most. Corrected in place; nothing
renumbered.

The §4 sweeps were run before and after against a pristine checkout of the base commit with its own
build directory, both closed afterwards. **The ninth sweep paid on this round's own writing**: the
verdict table had filed Issue #58 under Table 337 where `/Asset` is Table 343's, and `--bin tables`
moved its *absent* count by one and named the pair. Corrected, and every finding count across the
thirteen sweeps then matched the baseline exactly — pointers absent, tables absent and contradicted,
quotations diverging on both populations, and the ledger's own arithmetic. The remaining deltas are
volume: a document was added, so there are more pointers and more quotations to count.

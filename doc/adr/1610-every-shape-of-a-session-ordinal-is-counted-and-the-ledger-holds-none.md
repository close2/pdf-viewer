# 1610 — Every shape of a session ordinal is counted, and the ledger holds none

Session 1387. Status: **accepted**. Completes ADR 1547 and amends what its test counts; amends
nothing else. Context: `tools/conformance/tests/ledger_notes.rs`; ADRs 0281, 1023, 1535, 1547.

## The question

ADR 1547 holds the ledger's session ordinals by equality and lets the families come off one at a
time. Its test counted two shapes: a hyphenated ordinal holding `hundred` or `thousand`, and
`session` or `round` followed by digits. It left a small ordinal before `session` "to the reader".
Clauses 7 to 12 had reached zero by that count, and their notes still read "as of the
twenty-seventh session", "reviewed as a family in the twenty-eighth session", and "argued for
eighteen sessions". Those are the same counted fact. A zero that leaves them in is a zero about one
spelling.

## Decision

**The test counts four shapes.** The two it already counted. A spelled ordinal below a hundred
followed by the noun `session` (possessive included). A number, spelled or in digits, followed by
`sessions`. The noun is the line. Without it a small ordinal is a fraction ("a twentieth of a
pixel"), a position ("the twenty-eighth page"), or a sweep's name ("the twenty-second sweep"), and
`each_shape_of_ordinal_is_counted_and_a_fraction_is_not` pins both sides. A small ordinal before
`round` is not counted: in these notes "a second round" is an action raising another one (§12.6.3).

**The count is zero and held with `==`.** By the old rule the count was 297: clause 14 had 159, 10
had 100, O 23, D 5, 6 and I 4 each, and E 2. The extended rule found 63 more, so 360 by the
extended rule, spread over 12 families. Eight forked readers rewrote 119 rows from JSON copies.
That is the 105 rows carrying one, and 14 of the 33 `inapplicable` and `writer-side` rows, which
were re-read in ADR 1535's shape. The notes went from 425 217 to 350 077 characters. A checker held each rewritten note to
its old one: every double-quoted quotation, backticked name, ADR and `§` number kept had to be in
the old note, no shape could remain, quotes and backticks had to balance, and no `\u` could appear.
The new quotations in the `inapplicable` and `writer-side` rows were checked against `doc/md/`
instead. Each chunk was applied under `flock /home/AI/ledger.lock`, with `count == 1` asserted on
a fresh read, and `cargo test -p conformance` ran after it. `--bin quotations` still finds 3
diverging ledger quotations, the same three. No status moved.

**An elided ordinal is not counted, and none remains.** Notes such as "since the seventy-second"
and "after the twenty-ninth applied it" carry no noun, so no rule can tell them from a fraction.
The readers took them out of the rows they rewrote, and the checker listed every ordinal word left
in a rewritten note for a person to read.

## What it costs

A note may no longer say how long a sentence stood. Where that length was the lesson, it is in the
ADR the note cites. The test's four shapes are a word list. A spelled ordinal in a form the list
lacks would pass, and the list grows the day one is found.

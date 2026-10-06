# 1576 — A todo file states what is

Session 1370. Status: **accepted**. Extends ADR 1547's rule from ledger notes to `doc/todo/`'s
files, on ADRs 0281 and 1023's ground; amends nothing else.
Context: `CLAUDE.md` *Where knowledge lives*; ADRs 0281, 1023, 1416 (a done todo file's citers),
1547; `tools/conformance/tests/spelled_ordinals.rs`.

## 1. The rule

A todo file states **what is owed and why, as of now**. A done item becomes a sentence of what the
program does, with the ADR by number. If `doc/state-of-play.md` already says it, the item is
deleted. An owed item keeps its current reason. A sentence that says which round found, built or
measured something is chronology, and `doc/history/` and `doc/adr/` keep chronology. A correction
is applied rather than appended ("this said X and was wrong" becomes the right sentence). A heading
names its subject, not the round that wrote it.

The reason is ADR 1547's, unchanged. A todo file is read to choose work. When a sentence opens with
the round that wrote it, the reader meets the history before the fact, and the cost of reading the
item becomes the history of the item. A file that is a **record** keeps its chronology:
`doc/history/`, `doc/adr/` and `doc/reviews/`. So does an incident that carries a rule (a trap, a
habit, and the incidents `doc/todo/02` cites as the reason for a rule). In those the incident
stays, and only the ordinal goes ("one round rebuilt the GTK host three times").

## 2. The figure, printed and not held

`spelled_ordinals.rs`'s third test, `the_todo_files_spelled_ordinals_are_counted`, prints the
lines under `doc/todo/` that spell a round by ordinal, file by file, with the total. It asserts
nothing. Batch fifty-seven started at 1109 lines. `00`, `02`, `03` and `38` went to zero in this
round. Rounds 1365 and 1369 rewrote `30`, `31` and `01` in the same batch.

## 3. What enforcing it would take

1. **The rest rewritten.** After batch fifty-seven, about thirty files carry the remainder. `23`,
   `11`, `_scan-conversion`, `48` and `13` carry the most. That is three or four batches at four
   files a round.
2. **Then a ratchet, then a ceiling.** First a test that the count only falls, against a figure
   written in the test, as `ledger_notes.rs` held the ledger. Then `CEILING == 0` once it reaches
   zero, with `==` so that a fall is written down rather than banked (ADR 1535).
3. **The digit form beside it.** `session 995` and `round 913` are the same chronology in digits.
   The source gate's `Session <digit>` shape misses them, because a todo file writes the word in
   lower case. The pattern to hold is the word `session` or `round` followed by a number. `todo/02`
   has a dozen of these, kept in this round where they are an incident's evidence. Those would need
   rewording or an exemption line before the digit form can be held.
4. **No exclusion is needed** for the todo files: none is a record. A done file kept only for its
   citers (ADR 1416) states its done item in the present tense like any other.

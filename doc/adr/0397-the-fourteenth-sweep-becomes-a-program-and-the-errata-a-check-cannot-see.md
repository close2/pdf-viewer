# ADR 0397 — The fourteenth sweep becomes a program, and the errata a `check` cannot see

Status: accepted.
Session: the five-hundred-and-sixty-second, a sweep round under `doc/todo/01`'s binding rule.

## 1. What this decides

Three things, and the second is the one worth arguing.

1. **`doc/todo/01`'s fourteenth sweep is `conformance --bin owed`** — the twelfth of the fifteen to
   become a program, and the last of the four whose printed *level* moved with the session rather
   than with the tree.
2. **Its discriminator is not a debt vocabulary at all.** A vocabulary learned from the ledger was
   tried and does not work; what replaces the word list is a measurement against the tree, and it is
   the seventh sweep's own measurement with the sign reversed.
3. **`spec-errata check` is one direction of one question, and `emit` is the other.** A round that
   wants to know whether an erratum has moved ground the ledger stands on cannot ask `check`, and
   `doc/todo/02` §4 now says so with an invocation.

## 2. The sweep, and what was wrong with it as a description

The ledger defines `partial` as "some are; the note says which, which are not, and what is
reported". A row that is `partial` above a note naming nothing owed breaks that definition, and no
other sweep in `doc/todo/01` can see it: sweeps 1, 3 and 4 read a *reason* — a blocker, a capability,
a retired string — and a row in this shape gives none; the sixth's arithmetic asks whether every
child is settled, and a family carrying this error usually disqualifies itself with it. Its first
run, in the four-hundred-and-thirty-seventh, moved §9.3 out of a `partial` it had held for 365
sessions.

**As a description it printed 16, 24, 5, 15, 19, 10, 8, 9 and 19 hits over nine runs**, across
rounds that moved almost no rows, because each session wrote the debt vocabulary that morning:
"writer-side", "may be ignored", "is not read", "counted, not parsed", "aggregate of the two below".
ADR 0360 priced that failure for the caller sweep and ADR 0388 for the seventh: **a level that
cannot be compared with the one before it says nothing at all.**

## 3. The learned vocabulary, tried and rejected

`doc/todo/01` had named the remedy: "the seventh sweep's own answer is available to it — a
discriminator taken from the ledger instead of from memory." Read as *learn the vocabulary from the
ledger*, that is a natural design and it fails, and the failure is worth writing down so that nobody
builds it twice.

The construction: take the notes of the rows that owe something (`partial`, `reported`) as one
corpus and the notes of the settled rows as another, and call a word a debt word where it is much
likelier in the first than in the second. Measured over this ledger, the words that come out on top
are

> `digest`, `stored`, `isolated`, `ap`, `caret`, `appearance`, `knockout`, `signature`

— which is not debt language. It is a list of **subjects**: signatures and annotations are mostly
`partial` and lexing and filters are mostly `implemented`, so the contrast measures topic. Status
and subject are not independent in this ledger, and no reweighting inside one ledger separates them.

Widening the candidate set makes it worse rather than better. At every threshold tried — a word in
5%, 8%, 10%, 15% or 20% of owing notes, at ratios from 1.4 to 3.0, with and without a requirement
that the word span half the clause families — **every one of the 228 `partial` notes holds at least
one word of the resulting lexicon**, so the hit count is zero and the instrument says nothing. A
ledger note is a paragraph, and a paragraph of any length contains some general word.

## 4. What replaces it, and why it is the seventh sweep's answer rather than a new one

The seventh sweep did not learn a vocabulary either. It replaced a stop-list — a guess about which
words are *common* — with a **count taken from the tree**: how many sources name a term the row
itself states. The same measurement answers this sweep's question when it is read from the other
end.

| | population | the term that matters |
|---|---|---|
| seventh sweep (`inapplicable`) | the row claims the tree does not do this | one the tree **names** |
| fourteenth sweep (`partial`) | the row must say what is *not* done | one the tree **lacks** |

The argument in one sentence: **a note that names a debt names a thing, and a thing this tree does
not have is a name no source carries.** The extraction is shared with the seventh sweep verbatim —
`inapplicable::terms_of`, `inapplicable::names`, the two derived rules that make an identifier out of
an inner capital and out of a `Capitalised` word that does not open a sentence — so the two sweeps
disagree about nothing except which end of the count they read.

So the reading list is every `partial` row **none of whose own vocabulary the tree lacks**: a row all
of whose named things this tree already has, sitting above a status that says something is missing.
And the same count orders it: a row is ranked by its **rarest** term's reach, commonest first,
because a row whose most distinctive word is still named by a hundred files has named no *thing* at
all. A row stating no name whatever heads the list, which is why an absent minimum is the largest
value there is rather than the smallest.

The level is now a property of the ledger and the tree. First run:

> 225 `partial` rows stating 2983 terms: 159 named by no source — a debt named in a word — over 105
> rows, leaving 120 rows whose every stated term this tree already names.

## 5. The noise, and why it is printed rather than filtered

- **A debt named in prose with no identifier in it.** "Conditional on importing the target page",
  "defers each to a subclause", "the vertical branch of the displacement formula". This is the same
  noise the hand-runs recorded, and no word list removes it either: what a program can settle is
  whether a *name* is missing, never whether a sentence means one is. Half the population lands here,
  which is why this is a reading list and not a gate — ADR 0249's ratio argument, unchanged.
- **One short key, three clauses**, `doc/todo/01`'s oldest false positive: a key the tree names under
  another table reads as present, so the row loses a term that was a real debt.
- **A row narrating its own correction** reproduces the retired vocabulary, which every sweep here
  has.

**And §9.3.1's inverse is untouched**, which the four-hundred-and-thirty-seventh already established
and which this design does not improve: a row that names a debt and is *wrong* about it is invisible
to any instrument that only asks whether a debt is named. That is a property of the question rather
than of the vocabulary, and widening anything makes it no better.

## 6. Its first finding as a program

**§12.7.6.1 was `partial` above a note naming nothing owed**, and it is at the top of the rarest-first
order because its note states one term. The clause is three bullets and a sentence:

> Interactive forms also support special types of actions in addition to those described in 12.6.4,
> "Action types":

followed by submit-form, reset-form and import-data. There is no `shall`, no entry and nothing a
processor can fail; what each action *does* is §12.7.6.2's, §12.7.6.3's and §12.7.6.4's, and those
rows carry their own statuses. A framing row is `implemented` when its own prose is executed, which is
this ledger's convention wherever a parent's children still owe something — §7.4's framing beside five
`partial` filters. **`implemented`**, and with evidence rather than an assertion:
`each_of_the_three_form_action_types_reaches_its_own_answer` asserts that all three names arrive
somewhere deliberate and that the third is refused *by name*, which is the distinction a reader
dropping `SubmitForm` on the floor would pass without.

## 7. The errata, from the other end

The five-hundred-and-fifty-fifth session found ISO/TS 32001 §5.1.3 deleted by erratum #236 while this
tree had been asserting its content since ADR 0314 — in code, in three ledger notes and in a todo
file — and wrote the rule into `doc/errata-read.md`: a round implementing a clause runs `emit` on that
document before it writes. **This round asked what running `emit` over everything finds**, which is a
different question from the one `check` asks.

`check` compares *the tree's quotations* against struck passages. It is blind, by construction, to an
erratum over text nobody has quoted — and a heading is not a sentence, so it is blind to every
structural change there is. `emit` prints 1097 annotations over the three documents in `doc/` that
carry any; twenty are structural, and three filters find them: the words *delete*, *move* and
*renumber* in an editor's note, and a strikeout whose whole text is a requirement word.

**Two of the three renumbering errata were unrecorded.**

- **Issue #452** (`Review`/`Completed`): "Move entire subclause 14.7.5.1.1 up one heading level to
  become 14.7.5.2 and renumber later subclauses of 14.7.5 appropriately. Subclause text is otherwise
  unchanged." Five ledger rows and some twenty source citations carry the 2020 numbers.
- **Issue #196** (`Review`/`Completed`): inserts "7.6.5.3 Public-key security permissions" below Table
  23's NOTE, "current text and Table 24 remain unchanged" — so the existing §7.6.5.3 takes the number
  after it.
- **Issue #133** is the same shape and was read in the four-hundred-and-thirty-seventh (ADR 0273). An
  instrument agreeing with a known finding is what says it works.

**The numbers are not changed anywhere, and that is a decision rather than an omission.** `doc/md/` is
the published text, the citation gate resolves against it, and
`the_ledgers_own_prose_names_clauses_and_tables_that_exist` refuses a post-erratum number outright —
confirmed by writing two of them and watching the gate fail. What changes is that the three families
now say so in their own notes, which is the only form of the correction that survives regeneration.

### 7a. And one erratum that changes a requirement rather than a number

Filtering the same output for a strikeout whose whole text is a requirement word printed nine pairs,
of which **Issue #22** (`Review`/`Completed`) is not read anywhere: Table 166's `/AP` goes from
"Optional; PDF 1.2" to "Required except for conditions listed below (PDF 2.0); optional in PDF 1.2
through PDF 1.7", the conditions being a degenerate `/Rect` and a `/Subtype` of `Popup`, `Projection`
or `Link`.

**The 2020 text already said it in prose**, which is the part that matters: "[a] PDF writer shall
include an appearance dictionary when writing or updating the PDF file except for the two cases listed
below." So the erratum moves a requirement into the column it belonged in — and two doc comments in
`pdf-model/src/view.rs`, the file that writes annotations under §7.5.6, said "an annotation with no
`/AP` is legal", which was false before the erratum as well as after.

Both are corrected, and the one place this program departs from the `shall` is argued in place rather
than tidied away. `write_retypings` removes a producer's stored appearance for a free text annotation
whose new text this program could not lay out, because Table 177 makes `/AP` decisive over `/DA` and
leaving the old stream would draw words the file no longer states. The cost is a PDF 2.0 annotation the
amended table calls non-conforming; the alternative is a page showing text nobody claims. **It is
reported** — `Written::unappeared` names every such annotation — which is what makes it a departure
under `CLAUDE.md`'s first principle rather than an oversight.

## 8. Consequences

- `cargo run --release -p conformance --bin owed` is a line in `doc/todo/02` §4, and so is
  `spec-errata -- emit doc/*.pdf` beside the `check` that was already there.
- `inapplicable::Kind` gains `Ord` so that a term can key the reach cache the wider population needs;
  nothing else about the seventh sweep changes, and its numbers this round are unmoved.
- **Sweep 10 is the last whose level is session-local**, and `doc/todo/01` names it as the next one a
  program should take over. It needs no discriminator at all: the count is in the sentence and the
  family is in the file.

## 9. What this does not decide

Whether a `partial` row's debt named only in prose should be named with an identifier instead. It
would make this sweep sharper and it would be a rewriting of a hundred notes for the instrument's
benefit, which is the wrong direction — the ledger is written for a person.

# ADR 0400 — The tenth sweep becomes a program, and the number an erratum moved

Status: accepted.
Session: the five-hundred-and-sixty-fifth, a sweep round under `doc/todo/01`'s binding rule.

## 1. What this decides

Three things, and the third is a standing answer rather than a program.

1. **`doc/todo/01`'s tenth sweep is `conformance --bin counts`** — a parent row's stated count of its
   family against the rows below it — the thirteenth of the fifteen to become a program and the last
   of the ten whose printed *level* moved with the session.
2. **Its discriminator is two measurements this project already has, and neither is a vocabulary of
   counting.** The ninth sweep's attribution decides which numbers are claims about a family; the
   sixth sweep's arithmetic answers them.
3. **A clause number the errata have moved is recorded and not renamed, and the finding of one is a
   command.** `spec-errata moved` prints every erratum that moves a number with the ground this tree
   has standing on it, and its first run found two the previous round's hand-filter walked past.

## 2. The sweep as a description, and why its number said nothing

The tenth sweep was invented by the three-hundred-and-ninety-fourth session and built by the
four-hundred-and-second, and what it looks for is `doc/todo/01`'s fifth failure shape at family
scale: **a parent row is not maintained by the sessions that implement its members**, so a row that
says "three of the twenty" is making a checkable claim nobody checks. Its findings have been large.
§12.7.6 said "the other two are refused by name" for **280 sessions** while three other rows held the
right answer; §11.7 stated its own family's count twice, four sentences apart, disagreeing, for
**410**; §14.8.2 said "[o]f the twelve rows below" over thirteen, and the count was wrong on the day
a *sweep round* wrote it.

**Ten hand-runs printed 16, 185, 124, 10, 160, 17, 70, 25, 41 and 4 counted claims**, over a ledger
whose families barely move. That spread is not the ledger drifting; it is ten patterns written on ten
mornings — one round counted every digit in a note, another only the phrase "aggregate of the N
below", a third a number word beside a verb of implementation. A level that cannot be compared with
the one before it says nothing at all, which is ADR 0360's argument for the caller sweep, ADR 0388's
for the seventh and ADR 0397's for the fourteenth.

## 3. The discriminator, and the shape ADR 0397 asked us to look for first

`doc/todo/01` predicted that this sweep "needs no discriminator at all — the count is in the sentence
and the family is in the file". **The second half is right and the first half is the whole problem**:
the family is in the file, and *which numbers are about it* is what every hand-run guessed at. The
obvious answer — a vocabulary of counting verbs — is exactly what ADR 0397 found wanting for sweep
14, where a lexicon learned from the ledger measured **topic** rather than debt. So the shape to look
for is another sweep's measurement, and here there are two.

**The ninth sweep's attribution decides the population.** `tables` counts a key as a claim about a
table only where the sentence *attributes* it, because a sentence naming a table usually goes on to
name the dictionary the table describes. The same rule, one subject over: a cardinal is a claim about
a family only where it **governs one of the ledger's own words for a row** — `row`, `subclause`,
`child`, `below` — within three words and without crossing the sentence's own punctuation. That one
rule removes the noise every hand-run printed and named: "three of the clause's properties", "two
entries that would add to a document", "[f]our of them are stream filters" are all counts of the
standard's furniture rather than the ledger's. And the container is the clause the sentence
**names**, exactly as a table is — which is how §12.6.3's count of §12.6.4's family, invisible to
every hand-run and found by the blame band in the five-hundred-and-twenty-fifth, becomes a claim this
program can judge at all.

**The sixth sweep's arithmetic answers it.** A `Family` publishes every cardinality its own rows can
produce: its direct children, its descendants, each of those without the family's `General` row, and
each with the clause's own row counted in — plus, for a part, the count of children and descendants in
each status. So the two conventions this ledger keeps are *derived* rather than remembered: five
parent rows count a family without its `General` row (§14.10.3, §14.10.4, §14.8.4.7, §7.6.4.4,
§14.8.2), and this project writes "§11.7 — fourteen rows" and "§7.6, a clause family of 34 subclauses"
with the clause's own row inside the family. Both were noise on the first run and both are agreement
now, because the arithmetic says so and not because a reader remembered.

**The sign is reversed from the sixth sweep in the way that matters.** The sixth reads the family's
statuses to judge the parent's *status* — a row that owes something above children that all owe
nothing. This reads the same numbers to judge the parent's *prose*. One instrument, two ends, and
neither is a word list.

### What a program adds that no grep could

- **A contradiction needs no family at all.** Two claims in one place, about one family, with the same
  noun and different numbers, are wrong whatever the ledger holds — and this is the only check here
  whose evidence is entirely inside the prose. It is required to span **two sentences**, because one
  sentence naming two numbers is a phrase with two roles in it ("one of the fourteen rows", "19
  unreviewed rows out of 20") and the shape this looks for is the sixth failure shape: a count
  appended to a paragraph whose first half nobody re-read, "four sentences later" in §11.7's own
  words.
- **Two rungs are counted rather than printed.** An agreement says *which* cardinality it is, so the
  convention is legible; and a count attributed to a clause with no rows below it is counting
  something the ledger does not hold.
- **A correction is marked, not dropped** ([`retired::kind_of`]), because this project writes the
  retired number into the sentence that retires it.

### Three rules the first run taught, none of them about counting

Each removed a false cardinal, and each is about the sentence rather than about the subject:

- **`below` is an ellipsis and needs the number immediately in front of it.** "Its own table twelve
  lines below" counts *lines*.
- **A cardinal's reach stops at a semicolon, a colon or an em dash.** "It named three of the ten;
  §12.6.4's row carries the count" counts action types and then mentions a row.
- **A number with a full stop or a leading zero in it is not a quantity.** `§9.6` reduced to its
  digits is `96` and `ADR 0027` is a name — both of which the first run reported as counts of
  somebody's family.

### The first run

**170 clauses have a row below them; 5184 sentences govern one of the ledger's own words for a row;
296 attributed counts — 125 the family agrees with, 45 it can be counted no such way, 126 attributed
to a clause with no rows below it; 3 places count one family twice.** No defect in the ledger, and the
evidence that the instrument works is its own agreement with four known findings: §11.7's double
count, §14.8.2's twelve over thirteen, §12.6.4's and §12.7.6's all come back as `[correction]` hits on
the very numbers their rounds retired. A clean first run is a result — it says a population is not
drifting — and this one is over the population thirteen consecutive sweep rounds have been reading by
hand.

**It is not a gate**, for ADR 0249's ratio reason. Three noise shapes survive every rule above and all
three are correct English: a count of the standard's subclauses rather than the ledger's rows, a count
of the rows a `General` row sits *beside* rather than the family below it, and a count qualified by a
status ("the two rows below it that are `writer-side`"), which is a part with its denominator elided.
Admitting a status count for a bare number would make the accept-set of a small family almost
everything, which costs the denominators every defect so far has been in.

## 4. The errata a hand-filter cannot see either, and what this tree does about a moved number

The five-hundred-and-sixty-second read `emit`'s 1097 annotations by hand, found Issues #452 and #196,
and wrote down the three words to filter for. **A filter written down is a filter somebody
re-invents** — `CLAUDE.md`'s own rule, and the failure ADR 0319 priced when the fifteenth sweep went
unrun for twenty-four rounds — so it is `spec-errata moved` now: every annotation whose instruction
uses `move`, `renumber`, `delete` or `insert` **and names a clause number**, with what this tree has
standing on that number beside it.

**Its first run found two the hand-run missed.** Issue #477 (`Completed`) moves all of §12.3.6 down a
level, which is the same shape as #452 and was missed for a reason worth keeping: this collection
writes an instruction in two voices, and #477's is the past passive — "all of subclause 12.3.6
Navigators **was moved** and demoted one level" — where #452's is an imperative. A command that takes
the verb *and* the number finds both; a person filtering by eye finds the one that reads like an
instruction. Issue #256 (`Completed`) is a different kind and the more interesting: it changes no
number and no sentence, and says §12.6.4.8's `/Base` text "applies to all relative URIs in a PDF
document and is not limited to only URI actions as is currently implied", with the move deferred to "a
future edition". Nothing is owed by the erratum; the *reading* is owed now, and `uri::resolve` is
already general — RFC 3986's reference transformation over a base and a reference, knowing nothing
about actions. Its one caller is this clause's action, and the other relative-URI site this tree reads
is §7.11.2.2's URL-based file specification, which is validated and not resolved because nothing here
fetches a URL. That is a documented choice with the erratum beside it, and a second caller the day a
host opens one.

### The standing answer: recorded, not renamed, and findable by command

A citation that is right against the published text and wrong against the amended one is a real
hazard for a reader, and this round was asked to decide what to do about it. Three parts:

- **The published numbers stay, and this is compiler-enforced rather than preferred.** `doc/md/` is
  the text every `§` resolves against, and `the_ledgers_own_prose_names_clauses_and_tables_that_exist`
  refuses a number ISO 32000-2 does not have. §12.3.6's new note was written with a section sign in
  front of the amended number and **failed that gate** — the second round running to confirm the rule
  by walking into it. The amended number is therefore written as the erratum writes it, *subclause
  12.3.5.3*, with no section sign, and a renumbering cannot quietly enter the tree as a citation.
- **The row is where the amendment is recorded**, because a row is what a round reading a clause
  opens. Four families now say so in their own notes: §14.7.5.1.1, §7.6.5.2, §12.3.6 and §12.6.4.8.
- **The command is what makes it findable**, because a note in one row is not read by the round that
  writes the twentieth citation in `crates/`. Nothing is renamed, nothing is deprecated and no comment
  is annotated: a citation of §12.3.6 is correct about the standard this project is checked against and
  *incomplete* about a reader holding Errata Collection 3, and one command closes that gap in a second.

**What was rejected, and why.** Renaming the numbers is refused by the gate and would put this tree at
odds with the only copy of the standard it can check anything against. Annotating every affected
comment was rejected on cost and decay: twenty citations of §14.7.5's family and seven of §12.3.6, each
a sentence to maintain, against one command that derives the same list from the PDFs. A marker in the
ledger row *alone* was rejected as insufficient rather than wrong — it is kept, and it is not read by
the person the hazard is about.

## 5. Consequences

- `tools/conformance/src/counts.rs` and `src/bin/counts.rs` are new, with ten unit tests; the module
  documentation carries the argument above and the noise shapes.
- `tools/spec-errata` gains `structural`, `clauses_named`, `standing_on` and the `moved` subcommand,
  with two unit tests.
- `doc/todo/02` §4 gains both commands, and the `emit`-by-hand instruction becomes `moved`.
- `doc/errata-read.md` carries the first run, the two errata and the standing answer above.
- Two ledger rows are corrected on the blame band and one status moves; §12.3.6 and §12.6.4.8 record
  their errata. `doc/todo/01` carries the run and the band.
- **Only sweep 6 is left as a description**, and it is two hits long and has never printed anything
  else.

# ADR 0502 — The entry a row declared unread, and the erratum nobody recorded

Status: accepted, 2026-08-23. Session the six-hundred-and-seventy-sixth, a clause round under
`doc/todo/01`'s binding rule, continuing the seventh step of its technique. Amends §12.7.5.5's,
§12.8.2.2's and §12.8.2.2.1's ledger rows, moves §12.7.5.5 from `implemented` to `partial`, adds two
claims and a tally to `examples/absence_audit`, adds one row to `doc/errata-read.md`, and corrects
three counts and a four-way grouping in `doc/todo/01`. Extends ADRs 0490, 0493 and 0496; changes
nothing ADR 0212, 0284 or 0403 decided.

## 1. What this decides

`doc/habits.md`'s decay shape — **a negative claim decays when the population grows** — was applied
to the two rows the previous round named as next, and both turned out false. What this round adds
beyond two more counts is a second decay shape and one instrument repair:

- **A negative can also decay into a clause nobody has read.** §12.7.5.5's row disposed of Table
  236's `/P` in one sentence and that sentence stood for many rounds; the entry has seven, and three
  of the other six are addressed to a processor that *changes* the file. The disposal had a second
  cost that nothing could have printed: because the row said the entry was not read, the errata
  collection had no reason to look at its page, and **an erratum amending that very entry has never
  been recorded here**.
- **A truncated witness list is a count with the finding cut off it.** `absence_audit` printed twelve
  names and "… and N more". §12.2's ninety-six witnesses were retired by a *distribution* rather than
  by a count — all of them stating Table 147's own default, which is what kept the sharper half of
  that claim true (ADR 0496 §5) — and a truncated list cannot show one. `report` tallies the distinct
  answers now, wherever it truncates.

## 2. §12.8.2.2.1's `/DocMDP`, re-derived

Curated is 1251 documents; `CC-MAIN-2021-31` is 65 944. Both populations, separately, per ADR 0490's
control-and-growth rule.

| | curated | crawl |
|---|---|---|
| documents whose `/Perms /DocMDP` binds a level | **1** | **143** |
| of those, Table 257's `/P` 1 — `Modification::None` | 0 | **122** |
| `/P` 2 — `Modification::FormFilling` | 1 | 16 |
| `/P` 3 — `Modification::FormFillingAndAnnotation` | 0 | 5 |
| a `/Perms /DocMDP` this tree reads no level out of | 0 | **0** |

**The distribution is the finding and the count is not.** The row's old sentence — "[o]ne corpus
document names a `/DocMDP` and it states `/P 2`" — is true of the corpus and reads as a claim about
the world, and the level it names is the *middle* one. In the world the majority level is 1, which is
the branch that withholds both of this program's operations and the one no file exercised: 122 real
documents assert what a single hand-built fixture used to stand for.

**The question the previous round told this one to ask first was whether anything is owed at all**, and
the answer is no, which is worth stating as a result rather than as a silence. §12.8.2.2's `shall`s
about *creating* a certification signature fall on a producer. The one that reaches a reader is
§12.8.2.2.1's parenthesis — changes "shall also be prevented if the signature dictionary is referred
from the DocMDP entry in the permissions dictionary" — and it has been honoured since the
hundred-and-ninety-first session, in the shape `CLAUDE.md` §3 asks for: `restriction::asserted` states
*which* level withholds *which* operation and returns a reason rather than a verdict, `viewer_core`
asks once per operation, the refusal reaches a person as `Event::Refused` carrying the clause, and a
host can turn it off. Nothing about the 143 documents changes that; what they change is that the code
is now known to be exercised rather than assumed to be.

**One negative in this row survives its own re-derivation, and it is worth more than the count.** Not
one document in either population states a `/Perms /DocMDP` whose level this tree cannot read — which
matters because the standard describes that `/Reference` twice and differently. Table 255 makes it
"An array of signature reference dictionaries"; Table 263 says the dictionary "shall contain a
Reference entry that shall be a signature reference dictionary", singular. `signature::modification`
accepts only the array, so a producer following the second sentence would be read as asserting
nothing at all — the failure would be silent and would run in the permissive direction. `doc/habits.md`'s
planted-witness rule is what makes the zero believable: a hand-built file with a bare-dictionary
`/Reference` was dropped into `doc/corpora-own` for one run, scored the answer that names exactly
that shape, and was deleted. **The zero is a measurement, not an absence of one**, and it is why the
array requirement is left alone: relaxing a reader on a shape no file in 67 195 documents has is
speculative work on the permissive side of a permission.

## 3. §12.7.5.5's Table 236 `/P` — a disposal that quoted one sentence of seven

The row read: "Table 236's `/P` is deliberately not read here: 'absence of this key shall result in
no effect on signature validation rules' makes it a statement about what invalidates the signature."
The `overstated` sweep has surfaced this denial for four rounds running and each round called it
noise, correctly — the *hit* is noise, because the parent row it is compared against asserts a
different table's entry. The denial underneath it is not noise, and nothing was looking at it.

The entry's other sentences:

> The access permissions granted for this document.

> That is, permissions can be denied but not added.

> If the document does not have an author signature, the initial permissions in effect are those
> based on the number 3.

> The new permission applies to any incremental changes to the document following the signature of
> which this key is part.

Those describe a permission regime over incremental changes — which is what this program makes when
it fills a field and saves — rather than a rule about validating a signature. And **errata issue #131
amends this entry** to carve out an incremental update carrying only a DSS or a document timestamp,
which is the same exception §12.8.2.2.1 states for `/DocMDP`'s `/P` 1. An erratum that carves an
exception out of a permission is evidence that the entry states a permission.

**The population, measured rather than assumed**: 28 of the 65 944 crawled documents state Table
236's `/P`, 0 of 1251 curated, and every one of the 28 states `/P` 1 with an `Action` of `All`. The
block was proved against a planted witness before the curated zero was believed.

**What the 28 cost today is exactly one operation.** `All` already withholds every field from
`Operation::FillInForm`, so form filling on all 28 is refused whether or not the `/P` is read;
`asserted` consults a field lock only for that operation, so `Operation::Annotate` on those documents
is accepted while the level they state says no change at all is permitted.

### 3.1 Why this round measures it and does not implement it

The entry is two-voiced, and that is a property of the standard rather than of our reading. Its
permissions are stated with **no route that makes them binding on a processor**, where §12.8.2.2.1
gives `/DocMDP` exactly such a route through §12.8.6's permissions dictionary and says so in a
parenthesis written for the purpose. Choosing between "this is a permission a reader owes" and "this
is a statement about what invalidates a signature" is a decision, not a detail, and `CLAUDE.md`
principle 1 says a thing that cannot be done properly now is not started now.

The asymmetry decides the order. Reading it as a permission adds a **default refusal on 28 real
documents** for a sentence the standard may not be addressing to us, and `CLAUDE.md` §3 is explicit
that a document's restrictions over its reader are low priority and always switchable — so the
error that costs a reader something is the one to avoid taking in a hurry. Leaving it unread costs
one operation on 28 documents nobody has reported.

So the row moves `implemented` → `partial` and names what is not executed. When it is settled it goes
through `restriction::asserted` like the other five reasons, which is what keeps the four policy
levels reachable without revisiting anything.

## 4. The instrument: a tally under a truncated list

`report` prints the distinct answers with their counts whenever it truncates. It cost four lines and
it earned itself on this round's own two blocks — 122/16/5 for §12.8.2.2.1 and 28-of-28 `/P 1 on
/All` for Table 236 are both invisible in twelve names — and retrospectively on §12.7.5.5's, where
the run shows 90 locks against 89 `FieldMDP` transforms and the tally says which one document holds
the lock without the copy.

Each block's answer is worded as *what the document holds*, which is what makes the tally a
distribution rather than a histogram of file names. That was already the convention; this makes it
load-bearing.

## 5. Three counts in `doc/todo/01` that the command contradicts

The previous round wrote that of 45 rows carrying an absence sentence, "10 named the crawl before
this round and 11 more do now, leaving **24**", and gave a four-group breakdown said to add up. Run
against that round's own commit, the script printed directly above that sentence gives **7 before and
17 after, leaving 28**. The population, 45, is right; the split is not, and the four groups name 24 of
the 28 — §8.6.5.6, §8.11.4, §9.7.4.2 and §9.10.2 appear in no group.

Three of the four are the grep's own sentence boundary rather than a claim: the regex ends a sentence
at any full stop, so a clause number or a file name inside one splits it. The fourth, §9.7.4.2, is
real and belongs with §8.4.3.5 and §12.5.4 — a row whose negative already has a census
(`hollow_glyph_census`) that owes a `--crawl` argument rather than a re-reading.

**The lesson is the section's own, one turn further in.** `CLAUDE.md`'s rule is that a fact which can
be counted is not written down; what is written down is the command that counts it. That rule was
obeyed here — the command is printed in the file, immediately above the sentence — and the sentence
beside it was still wrong, because the round carried the number forward instead of running the
command it had just written out. **A command in a document does not make the number beside it
measured.** The corrected paragraph therefore says to run the script rather than read the level.

## 6. What this does not decide

- **Table 236's `/P` is not implemented and not declared inapplicable.** §12.7.5.5's row is `partial`
  and states both readings with the evidence for each. A later round decides it with an ADR.
- **`signature::modification` still requires `/Reference` to be an array**, on the measurement in §2
  rather than on the absence of one.
- **§12.8.2.2.2's object-by-object comparison is still owed**, and §12.8.2.2's row now names where it
  would first be exercised: the crawl's 122 documents at `/P` 1, rather than the one curated `/P` 2
  whose digest does not match.

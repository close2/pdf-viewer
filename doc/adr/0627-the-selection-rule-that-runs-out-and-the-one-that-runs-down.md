# ADR 0627 — The selection rule that runs out, and the one that runs down

Status: accepted, 2026-08-25. Session the seven-hundred-and-thirty-fourth, a clause round under
`doc/todo/01`. Replaces the pairwise rare-sequence ranking (ADRs 0560, 0567, with 0579's and 0593's
rules for reading it) as the way a clause round chooses what to read. Adds a section to
`doc/todo/01`; changes no code and no ledger row of its own — ADR 0628 is what its first use found.
Extends ADRs 0481, 0538, 0551, 0593, 0610 and 0620.

## 1. What ran out

ADR 0620 §1 reports the pairwise ranking with all three of its strongest pairs spent, the third
rule with nothing above rank 4, and rank 4 a tie broken by an older rule. That is a method at the
end of its head. Two things follow and only the second is a problem: a ranking is allowed to run
down its list, but **a ranking that self-reinforces cannot run down its list**, because the family a
round has just read scores higher for having been read. ADR 0593 §1 measured that with one
instrument over two bases: §12.5.2 ~ §12.5.5 went from 17 shared rare sequences to 21 in the round
that corrected both rows. So the score rises where the debt fell, and the cure — 0593's third rule,
*take the strongest pair the previous round named and did not read* — is a hand-maintained list of
exceptions with a finite supply. It supplied three rounds and ran out on the fourth.

## 2. What was measured before anything was proposed

Nine rounds are recorded, the six-hundred-and-ninety-first to the seven-hundred-and-thirtieth, each
naming the rows it found a defect in. That is a labelled set — **21 rows** that were `partial` or
`reported` at their own round's base — and it can be used the way trap 13 asks: take the candidate
signal, run it over the ledger *as it stood before the round*, and see where the defect sat.

Eleven signals were run that way, each scored as the mean percentile of a defect row in its own
ranking, where 50 is chance. `doc/todo/01` has the table. The result is a negative and it is the
reason this ADR exists:

- The best signal, a count of distinct §-references in a note, scores **38.6** — better than chance
  by a ninth, over 21 points.
- **The incumbent scores 48.3**, which is chance. That is not a fault in it: the pairwise ranking
  ranks *families* and never claimed to rank rows. But it means the obvious repair — read the
  pairwise ranking with read pairs excluded rather than skipped — would be choosing rows by a
  measure that does not discriminate rows.
- **The hypothesis worth having lost.** The recurring shape across those nine rounds is a
  correction that reached one sentence of a note and not another: ADR 0620's list audited three
  times, ADR 0600's tally left standing through two corrections, ADR 0610's row that never heard.
  A count of how many commits have rewritten a note should predict that, and it scores **46.0** —
  worse than the note's word count. ADR 0610 §3 had already said why in prose: the defect is a
  *conclusion*, and nothing in this tree ranks conclusions.

So no property of a note ranks the rows inside a family, and a successor built out of one would be
a guess with a table under it.

## 3. The rule

> **Rank each live ledger row by the errata annotations that fall on it whose issue number this
> tree names nowhere. Reassemble the issue from every clause `emit` files it under, and read the
> issue whole.**

It comes from reading the nine rounds rather than from a hypothesis about notes. **Eight of the
nine found something through `spec-errata emit`**, and ADR 0593 §1 states the mechanism without
naming it: *a pair that survives its reading has still chosen where to look*. Twice — the
seven-hundred-and-sixteenth and the seven-hundred-and-twentieth — the pair itself was clean or
nearly so and every finding came off the pages it had chosen. The ranking's whole contribution in
those rounds was a page number, and this rule supplies page numbers directly.

Three properties it has and the incumbent does not:

- **A finite, known population.** The collection's issue numbers are a closed set, so *how much is
  left* has an answer rather than a rank. At this base: 2840 annotations over 252 sections carry
  **356** distinct issue numbers, this tree names **115**, and **241** are named nowhere.
- **It runs down rather than up.** Reading an erratum records its number, which removes it. Measured
  over the nine bases, the population of unread issues landing on a live row falls monotonically —
  103, 100, 97, 94, 91, 90, 89, 86, 86 — about two a round, which is what nine rounds of reading
  errata is. Where the pairwise score rose on the family a round had read, this falls on the row.
- **Hits are defect-shaped.** An erratum this tree names nowhere is a specific sentence of the
  standard nobody here has read. It is not a suspicion about a note that a person then has to
  adjudicate, which is what ADR 0567 §7 gave as the reason its own ranking was not worth building.

## 4. That it has a head, and that nine rounds walked past it

The same reconstruction says so. §12.8.1, §12.5.2, §12.7.5.5 and §9.8.1 are in the top six at
**every one of the nine bases**, and the rows those rounds actually landed on ranked 1, 4, 8, 17,
17, 22, 32, 39 and 50. §7.6.6 is the one row that left the head, after the six-hundred-and-ninety-first
read two of its issues — which is the decay, visible.

A head that has stood for nine rounds is not a head the method is about to exhaust. Restricted to
issues that change the text — at least one `StrikeOut` or `Caret`, since an editor's note asks a
reader for nothing (ADR 0567 §6) — and to rows that are `partial`, `reported`, `silent` or
`unreviewed`, the population is **63 issues over 41 rows**.

## 5. Two limits, stated here rather than found later

- **Named is not read properly.** ADR 0621's finding was about Issue #619, which this tree names
  and whose four carets it had recorded as two. An issue can be in the *named* set and still be
  half-read, so the population is a lower bound on the debt and this ranking is blind to a misread
  erratum. The instrument for that one is ADR 0621's — read the issue whole, across every page
  `emit` files it under — and this rule's third clause is exactly that habit made part of the
  method.
- **It ranks errata, not defects.** Three of the nine rounds' headline findings had no erratum in
  them: ADR 0610's paid `shall`, ADR 0600's report keyed on less than its drawing, ADR 0620's entry
  left out of a list. What this rule chooses is where to look, and reading a row against its code
  is still the thing that finds those. It is not a replacement for reading; it is a replacement for
  a ranking that had run out of head.

## 6. Why it stays a recipe

`emit`'s output is derived from documents this project may not redistribute (ADR 0187), so a sweep
consuming it would take the file as an argument, the way `--bin quoted` and `--bin unpriced` take
the oracle's log (ADRs 0495, 0606). That is a real shape and it is not built here: what a program
would buy over the recipe is step 3's attribution of a heading to the nearest live ledger row,
which is twenty lines of arithmetic — and it is also the only step a person gets wrong, so a round
that runs this twice and finds itself re-deriving it should build it. `doc/todo/01` carries the
recipe as commands, with the one trap in step 2.

## 7. The trap in step 2, and it is not hypothetical

The population is *the issue numbers this tree names*, so the grep that produces it decides the
answer. A numeric character reference is a `#` followed by digits: `&#124;` is how a Markdown table
cell escapes a pipe, and it is the only one anywhere under `crates/`, `doc/` or `tools/` — two
occurrences, in ADR 0484's comparison table. A search for the bare number finds them and answers
*recorded*.

**Issue #124 is one of the two this rule's first use found unread.** One collision exists in the
whole tree and it is on the issue that went unrecorded, on a page a previous round had opened. So
the grep asks for `Issue #NNN` with its prefix, and the rule is written down beside the command
rather than left to be met again.

# ADR 0490 — The denominator a sentence does not name

Status: accepted, 2026-08-22. Session the six-hundred-and-sixty-third, a clause round under
`doc/todo/01`'s binding rule. Amends §12.4.4's, §12.4.4.1's, §12.4.4.2's, §12.6.4.11's,
§12.6.4.13's, §12.6.4.15's, §12.11.2's and §8.6.5.7's ledger rows, and four source comments, one message string
and ADR 0462's own body. Extends `doc/habits.md`'s "a negative claim decays when the population
grows"; changes nothing ADR 0135, 0230, 0316, 0470 or 0473 decided.

## 1. What this decides

**A cardinal in a sentence about this program carries a denominator, and where the sentence does
not name it the number decays without anything changing.** Two rules follow, and both are cheap:

1. **A count of what the program *declines* is derived from the code that declines**, not from the
   code that draws. Where one enumeration is partitioned two ways — shaped/unshaped and
   reported/not — a sentence must say which partition its number is from, because the two counts
   differ and a reader cannot tell which one is meant.
2. **A negative about a corpus names the corpus, and the crawl is now one of the populations it has
   to be measured over.** `doc/habits.md` already says a negative decays when the population grows;
   what this adds is that the growth has *happened* — `CC-MAIN-2021-31` is on this disk, whole, and
   a census over it is minutes — so a ledger negative measured before it is not merely suspect, it
   is unmeasured.

## 2. The first instance: five, four, and a style that is a cut

Table 164 states twelve transition styles. `viewer_core::transition` partitions them twice over and
the two partitions are not the same:

| | count | which |
|---|---|---|
| `frame` shapes a picture for it | 7 | `Wipe`, `Split`, `Box`, `Cover`, `Uncover`, `Push`, `Fade` |
| `frame` shapes nothing | **5** | the four below, **and `R`** |
| `note` reports it by name | **4** | `Blinds`, `Glitter`, `Dissolve`, `Fly` |

`R` sits in the second and not in the third on the table's own words — "[t]he new page simply
replaces the old one with no special transition effect; the D entry shall be ignored" — so a cut is
exactly what the file asked for and there is nothing to tell anybody about. The
five-hundred-and-fifty-third session found a note saying *five are reported*, corrected it in
§12.6.4.15's ledger row and in §12.4's, and recorded both in `doc/todo/01`'s band table.

**It corrected the two homes it was looking at, and eight others went on saying it.** The wording
was still standing in §12.4.4.1's row, in §12.11.2's row, in `pdf_model::navigation`'s module
comment, in `viewer_host::Clock::shapes`'s doc comment, in that crate's own test's doc comment, in
`doc/todo/32`'s status line — which is the file a round working on this clause *opens* — in ADR 0462
— and in
`pdf_model::requirements::unmet`'s `Kind::Transitions` arm, which is not a note at all but **a
sentence this program hands a host to show a person**: *"five of Table 164's twelve transition
styles are reported by name rather than drawn"*. A file asserting Table 275's `Transitions`
requirement was being told a false number by the reader that could not meet it.

ADR 0101's shape, for the second time in seven rounds — 657 found `/SM`'s denial living in two
places and drew the same lesson. What is new here is *why* the string was so easy to repeat
wrongly: **both numbers are true of something.** A round writing "five" was not careless; it was
counting the unshaped and describing the reported. A denominator that is never named cannot be
checked, and neither the ledger's gate nor any of the twelve sweeps reads a cardinal against the
function that produces it.

So the rule is stated at the point of derivation rather than as a warning: `transition::note` is
the function that declines, and a sentence about what this program declines is counted from that
function's arms. `frame`'s arms answer a different question and have a different total.

## 3. The second instance: a negative measured over 978 documents and written about the world

§12.4.4.1's row read, of §12.4.4's whole subject:

> Demand measured again in the three-hundred-and-ninety-third, over the *page tree* of all 964
> openable corpus documents and the 14 in `doc/` — 978 documents, 1971 pages — … **not one states
> a `/Trans`, a `/Dur` or a `/PresSteps`** … so the witness is hand-built.

Every word of that is true and it was written before the crawl was on this disk. Re-derived this
round, with the same instrument and both populations:

| population | documents | `/Trans` | `/Dur` | `/PresSteps` |
|---|---|---|---|---|
| pdf.js, the four submodules and `doc/` | 1133 | 0 | 0 | 0 |
| `CC-MAIN-2021-31`, whole | 65 703 that open | **276** | **86** | **1** |

The first row is the control and it matters: the old sentence is *not* wrong about its own
population, which is exactly why nothing in the tree could see it. The population grew and the
sentence did not say which one it was about.

**And the decay is not only in the negative direction.** §12.6.4.13's row states a *positive* count
— "one corpus document states this action", over 964 — and the curated population is 1133 now and
two of them do, the second having arrived with a submodule. Same rule, same repair: name the
population.

What the second row buys is not bookkeeping:

- **§12.4.4.2 has a witness that is not a fixture, for the first time.**
  `cc-main-2021-31/7680/7680405.pdf` states `/PresSteps` on four of its 39 pages, with Table 165's
  nodes carrying `/NA`, `/Next`, `/PA` and `/Prev`, and their actions are §12.6.4.13's
  `/SetOCGState` — a slide build whose states are optional content groups, which is precisely the
  construction `examples/presentation_fixture` was hand-written to stand in for (trap 8).
- **§12.6.4.15 has one too, and it is the same file**: 14 `/S /Trans` action dictionaries, all
  performed, every one of them stating `/S /Blend` — a name Table 164 does not define, so it is
  `Style::Unrecognised` and `transition::note` says so. The thirteenth case this reader keeps on
  purpose had never been seen outside a unit test.
- **§12.6.4.11's `/Hide` has files behind it, and that row says out loud what it costs**: "[t]his
  one changes what is drawn", three sentences before "[n]o corpus document states one". Still zero
  over the curated 1133; **2165 `/S /Hide` actions over 8 crawled documents**, all performed, one of
  them stating 1900 by itself.
- **The debt §12.4.4 is `partial` for is now ranked.** Of the four styles reported rather than
  drawn, `Dissolve` is asked for on 221 pages of 11 crawled documents and `Blinds` on 16 pages of
  4; **`Glitter` and `Fly` are asked for by no document at all**, against `Fade` on 596 pages and
  `Wipe` on 258, both drawn. A refusal no file reaches is a sentence nobody reads, and two of the
  four are that.

## 4. What a round owes

Nothing new is built. Two lines join the existing practice:

- When a note states how many of something this program declines, take the number from the arms of
  the function that declines, and say which partition it is — `doc/todo/01`'s reading list gains no
  sweep for this, because a sweep cannot know which of two true numbers a sentence meant.
- When a note states that no document does X, say **which** documents, and where the claim predates
  the crawl, re-derive it over `corpus-cache/safedocs/cc-main-2021-31` before repeating it. A census
  over the whole crawl is minutes: `presentation_census` chunked through `xargs -P 8` is under a
  minute, `refused_action_census` reads every object of all 65 944 in ninety seconds.

## 5. The instrument, before and after

ADR 0485's habit, run as it asks. Only `ledger.toml` was edited for this reading; the source and
this file move some of the sweeps for reasons that are not about the ledger.

| sweep | before → after | why |
|---|---|---|
| `overstated` | **8 contradicted, 7 marked, unchanged** | the parents corrected here assert nothing a child denies |
| `counts` | 369 → 371 attributed counts, **4 counting one family twice, unchanged** | two new cardinals, both under a clause with no rows below it |
| `quotations` (ledger) | 1754 → 1757, **1 diverging, unchanged** | three spans quoting *this project's* retired wording, correctly scored unrelated to the standard |
| `tables` | 2175 → 2187 key citations, 2019 → 2031 agreed, **6 denials contradicted and 97 absent, both unchanged** | §12.4.4.2's row now names Table 165's `/NA`, `/Next`, `/PA` and `/Prev`, and the table states all four |
| `owed` | 179 → 180 unnamed terms over 113 → 114 rows | **§12.6.4.15 left the reading list**, and §6 is about that |
| `entries` | 282 → 283 rows explaining themselves by an arrival, **182 reported over 49 rows unchanged** | §12.6.4.11's row now dates its re-derivation |
| `pointers` | 6971 → 6998 paths, **118 absent and 13 undefined, both unchanged** | the two crawled documents and the two censuses named all resolve |
| `unread`, `blockers`, `capabilities`, `inapplicable`, `callers` | unchanged | nothing gained names a new entry, blocker or capability |

## 6. A noise shape the fourteenth sweep did not have

`--bin owed` reads a `partial` row's vocabulary and asks whether the tree names each term; a term
no source carries is a debt named in a word. §12.6.4.15's row named every one of its twelve terms
before this round and does not now, because the term it gained is **`Blend`** — a style name out
of somebody's *file*, which this tree deliberately does not name anywhere, because
`Style::Unrecognised` keeps such a name as data rather than matching it.

That is a new shape for that sweep's noise, and it is worth stating because the obvious repair is
the wrong one: the fix is not to drop the name. A witness is not a debt, and removing the evidence
to hold a sweep's level flat would be the instrument choosing what the ledger may say. The row is
one line longer in a reading list and the reading takes ten seconds.

## 7. Consequences

- `doc/todo/01` gains one line naming the crawl as a population every corpus negative owes.
- `requirements::unmet`'s `Transitions` arm says *four* and cites the function it is counted from.
- The four styles are ranked, so a later round choosing between them has a number rather than an
  ordering of the table; nothing is implemented this round, because the clause still states no
  quantity for any of them and 620's rule leaves a refusal that rests on the standard where it is.

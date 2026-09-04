# Q28 — May a cost instrument spend memory in the program a person runs?

Asked by round 929, which built the floor `Q27` asked about for three of the seven walks and hit
this on the fourth.

**The number 27 was taken twice, and this file is where that is written down.** Round 927 asked
`Q27-cost-floors-for-the-other-seven-walks` on its own branch and round 926 asked
`Q27-a-font-the-file-does-not-carry` on `main`, on the same day; the two met for the first time in
this round's merge, and both are now in the directory. Nothing dangles — every reference to either
resolves, because each cites its own filename — but this README's "a number is never reused" is
broken by a pair of parallel rounds rather than by anybody's mistake, and renaming either would
falsify an ADR and a history file that already cite it. **It is left for the merging round or the
owner to settle**, and recorded here rather than fixed unilaterally, because a third state would be
worse than two. The lesson for the convention is the one the parallel-round agreements already
teach elsewhere: a counter shared between rounds needs an allocator, and `ls doc/questions/` is not
one when two branches cannot see each other.

## The question

The sharpest cost property in `pdf-syntax` is exactly countable:

> A filter chain runs once per stream per document, and the only thing that may make it run again
> is the cache having stopped holding what it produced.

The arithmetic is an identity rather than a measurement (ADR 0899 §3), and three of the four
counters it needs cost nothing. The fourth needs the number of **distinct** streams the cache has
ever held, and there is no route to that which does not remember every key — a set that grows with
the document rather than with `DECODED_BUDGET`, on the order of a megabyte for ISO 32000-2's 1023
pages, held for the life of an open document, **inside the shipped reader**.

So: may an instrument that exists to catch a cost regression itself cost memory in the program a
person runs? And if the answer is "only where it is small", who decides small — because the
alternative shapes each have a cost of their own:

1. **Always on.** Simple, one code path, and the count is true of the program people run. It is
   also memory nobody asked for, in the crate `CLAUDE.md` principle 2 is strictest about.
2. **Behind a Cargo feature or an environment variable.** Free in a release build — and then
   `doc/todo/02` §2's own rule bites: "a gate that turns a shipped setting off is measuring a
   configuration nobody runs" (ADR 0498). The floor would hold over a build that is not the one
   being shipped.
3. **Not at all**, and the largest corpus walk in the sequence keeps no cost floor.

## Why this cannot be settled without the owner

It is a principle-2 judgement, not a technical one. The owner set the memory budgets in this tree
personally — "below 10 MB is definitely acceptable, 1 GB is not" is why `readback::BUDGET` exists —
and the question is whether a *diagnostic* draws on the same allowance as the thing it diagnoses.
It also decides the general rule for every future instrument of this shape, which is why it is
worth an answer rather than a round's choice: `pdf-vfs` already carries two (`Vfs::questions`,
`Cache::forgotten`), and they are cheap only because a generation holds few subjects.

There is a real argument for **3** and it should be on the record: the corpus walk's expensive
call is already floored where it is a *spawn* (ADR 0898), the decoded-stream cache has never been
implicated in a regression, and a megabyte spent to watch for one is a tax on every reader.

The argument against it is `Q27`'s: this project gates correctness thoroughly and cost almost not
at all, and the one time that was paid for, the defect was invisible to every gate for four
sessions.

## What the tree does meanwhile

- **Nothing is blocked and nothing regressed.** `pdf-model`'s corpus gate prints a wall clock that
  nothing compares to anything, exactly as before.
- No counter was added to `pdf-syntax`, deliberately — including the three free ones, so that the
  decision arrives as one piece rather than as a half-built instrument.
- ADR 0899 §3 records the identity and the arithmetic so that whichever answer comes back, the next
  round does not re-derive it.

## Recommendation

**Option 1, always on, if and only if the set is bounded by a stated ceiling** — a few thousand
keys, after which the count stops being exact and *says so* in the report, the way `NOT JUDGED`
does for a clock ADR 0884 will not stand behind. A floor that can say "this document outgrew the
instrument" is honest and costs a known number of bytes; one that silently grows is the shape this
project already refuses for caches.

Failing that, **option 3 over option 2**: a floor that holds over a build nobody ships is worth
less than an honest gap.

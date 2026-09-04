# 0882 — A superlative is a claim with no anchor, and six sweeps each wrote one

Session 921. Status: **accepted**. The record of a sweep of the instruction files the project owner
asked for directly on 2026-09-04: *"please also go through our instruction files (CLAUDE.md,
readmes, ...). I guess we can compact them again. there is probably obsolete information in them as
well."*

## Context

The instruction files — `CLAUDE.md`, `doc/HANDOVER.md`, `doc/todo/README.md`,
`doc/todo/02-every-round.md`, `doc/environment.md`, `doc/habits.md` and the directory READMEs — are
what a round reads before it does anything. They are also the one population in this tree with **no
gate of its own**: the conformance gate checks their citations and quotations, `--bin pointers`
checks their file references, `--bin parts` checks their cardinals about this tree's parts, and
nothing at all checks whether a sentence about *the project's own working practice* is still true.
`doc/habits.md`'s ledger section already holds the general form of this — a claim decays, and the
ones with no instrument decay unobserved — and this round is that habit turned on the files that
state it.

## Decision

Ten classes of false claim were found and fixed; `doc/history/921` lists each with its evidence.
One of them is a *shape* rather than an instance, and it is what this ADR is for.

### The shape: a superlative has no anchor, so nothing can check it

`doc/todo/02` §4 described the sweeps that do not read a ledger row's stated reason. Six bullets,
one per sweep, each written by the round that built that sweep. **Every one of them opened with
"and it is the newest", and five were false**: `overstated` (18th), `overtaken` (19th), `quoted`
(20th), `unpriced` (21st) and `parts` (22nd) had each been overtaken, and `doc/todo/01` carried the
same phrase five more times.

Nothing could have caught it, and the reasons are worth separating because they are what makes the
class general:

- **It is a claim about this project's own list**, not about the standard (which the ledger and the
  quotation gate read), not about the tree (which `--bin parts`, `--bin callers` and the
  capability sweeps read), and not about a corpus (which `--bin undenominated` reads). No sweep's
  right-hand side is *the neighbouring paragraph*.
- **It is true when written.** Every one of those six sentences was correct on the day of its
  commit, which is the property `doc/habits.md` already names for a ledger row's reason and for a
  claim about the standard's silence. What is new here is that the *decaying agent is the writer's
  successor* — the next round to do the same kind of work is precisely what falsifies it, and that
  round is reading the bullet it is about to make wrong.
- **Checking it costs a read of every sibling.** An ordinal — *the twenty-second sweep* — is
  checkable against `doc/todo/01`'s catalogue in one `grep`. *The newest* is checkable only by
  reading all of them, which is the check nobody runs and which is why six rounds each wrote the
  phrase and none read the one above it.

**So: where a round is tempted to write *newest*, *latest*, *the last one added* or *the only one*,
the form that survives is the ordinal plus the pointer.** The ordinal is an anchor a later round can
verify without loading the population; the superlative is a statement about a population's *edge*,
and an edge moves whenever the population grows. The same reasoning retires "Five notes bind here"
over ten bullets and "the four things a round has got wrong" over five: a **count of a list written
beside the list** is the same failure in its cardinal form, and both were found in this sweep.

This is the ninth kind of decay this project has written down and the first whose subject is the
instruction files themselves. It joins `doc/habits.md`'s ledger section, and `doc/todo/02` §4 now
states each of those six sweeps as *ordinal, what it judges against, and the rule it leaves a
round* — a table, which cannot hold a superlative at all.

### Compaction, and the one place it was declined

The owner asked for compaction as well as correction, and the rule taken was `CLAUDE.md`'s own:
where two files say the same thing, one keeps it and the other points; where a paragraph has been
amended twice, the current sentence stays and the amendment's history goes to the ADR. Applied to
`doc/todo/02` §4 (the six readings now live only in `doc/todo/01`, which that section's own opening
sentence already declares the authority — *"that file is the reading, and this is the rule"*), to
`doc/HANDOVER.md`'s restatement of `doc/history/README.md`'s rule, to `doc/todo/02` §6a's
restatement of `doc/questions/README.md`, and to `doc/environment.md`, which stated the KDE package
naming twice and kept its `git stash` recovery recipe under a bullet about `git add`.

**`doc/todo/README.md`'s index was corrected and deliberately not shortened.** Two of its lines
were false — items 57 and 58 both named work that had landed — and fixing those is the whole of
what a sweep owed. Shortening the rest would trade something checkable for something smaller: each
line carries the item's ADR citations, `--bin pointers` reads them, and the file's own preamble
already routes a reader to the item file for the authority. A round that wants that file shorter is
proposing a change to what the index is *for*, which is an argument rather than a sweep.

**`CLAUDE.md` was not edited at all.** Three of principle 2's sentences are false against the tree,
and a principle reading false is a `Q` file rather than an edit — `doc/questions/Q25` states them
with their evidence and a recommendation for each. This is the same discipline `doc/todo/59` §5
already applies to principle 3's sandbox sentence, which round 920 owns.

## Consequences

- **A round adding an item to a list in an instruction file writes an ordinal, never a
  superlative**, and never a count of the list beside it. `tools/state.sh` counts what can be
  counted (ADR 0281); a paragraph does not.
- `tools/state.sh` now runs `optimize_corpus` and a `vfs` section, which is what made
  `doc/todo/02` §2's claim that it "runs the same sequence" true again. **The fix went on the
  instrument rather than in the sentence**, deliberately: a script that runs less than the sequence
  is a weaker gate wearing the sequence's name, and weakening the sentence would have hidden that
  rather than closing it.
- Two claims about `unsafe` in this tree were widenings of a true claim about one crate, in
  `doc/state-of-play.md` and in `crates/viewer-qt/src/bridge.rs`. Both now say what the test that
  holds them says. **A sentence moved from a crate's header into a document about the tree changes
  its scope silently**, which is worth watching for wherever `doc/state-of-play.md` quotes a module.

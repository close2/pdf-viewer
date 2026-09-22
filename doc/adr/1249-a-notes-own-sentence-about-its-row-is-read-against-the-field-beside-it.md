# ADR 1249 — A note's own sentence about its row is read against the `status` field beside it

## Status

Accepted, 2026-09-22. Session 1206. Binds `tools/conformance` and `doc/todo/01`'s catalogue.

`§N` is ISO 32000-2 and nothing else.

## Context

Every sweep in `doc/todo/01` reads a ledger row against something *outside* the ledger: the
standard's words, the tree's symbols, the disk's corpora, the decision records. None of them reads a
row against itself — and `doc/habits/the-ledger-and-claims-about-this-tree.md`'s newest decay shape
is exactly that comparison:

> A ledger note's last sentence — the "what keeps this row `partial`" clause — is the one most
> likely to be stale, because every later round appends above it.

Session 1200 found three by hand in one sitting. §12.11.6's *opening* sentence said what the row was
`partial` for while the field beside it said `implemented`; §8.11, §8.11.1 and §8.11.4 closed on a
debt that had been built; §12.5 closed on a print path that exists. The first of those is the shape
a program can see: a status word written in the note, in backticks, that is not the word in the
`status` field two lines above it.

## Decision

`conformance::last_sentences` reads each row's **opening and closing** sentence — the two ends, for
the two reasons they decay — and reports every backticked status word that is not the row's own.

Both ends, not only the last. The habit names the closing sentence because rounds append above it;
the opening sentence decays by the mirror route, because a round that moves a status rewrites the
paragraph it is working in and leaves the note's first line describing the row it used to be. The
live defect session 1200 found was an opening sentence, and a sweep reading only the other end would
have walked past it.

**Three rungs, and the middle one exists because the ledger's house style produces it.** A
correction in this ledger states the claim it retires, on purpose — that is the same habit file's
rule, so that a sweep whose population is a phrase can still find the row. So a self-referential
sentence with a past-tense marker beside it (*was*, *until*, *kept*, *said*, *no longer*) is marked
rather than dropped, and only a present-tense self-reference is the defect. Without the split, the
top rung is five hits and all five are the house style.

**It prints rather than fails.** Whether a sentence is a claim about this row is a question about
English; a gate that answered it would be believed, which is what `doc/habits/measuring.md`'s
ten-hits rule was written after. The assertion beside it in the same catalogue — ADR 1250's map
membership — *is* a gate, and the difference is that both of its sides are lists.

## Calibration

Trap 13, and by a plant rather than by a live defect: the three session 1200 found were corrected in
this same session, so the sweep's first run has one hit on its top rung and it is the documented
noise shape (§8.6.6.5's note closing on `doc/questions/Q100`'s recommendation, which is a sentence
about a future status and not a stale one). The plants are unit tests in the module: §12.11.6's own
opening sentence as it stood, named on the top rung; the same sentence as the correction now words
it, demoted to the middle rung; a parent naming a child's status, on the last; and a row whose note
writes its own word, not a hit at all.

## Consequences

- One more thing a round can run in a fraction of a second, and one decay shape that had no
  instrument now has one.
- The rung vocabulary is a judgement about English and will be wrong at the edges. It is written to
  be wrong in the cheap direction: a real defect demoted to the middle rung is still printed, while
  a filter would have removed it.
- The sweep cannot see a closing sentence that is stale about anything *other* than a status word —
  §8.11.4.1's, which closes on a debt that has been built and names no status, is invisible to it.
  That is the seventh sweep's population and not this one's.

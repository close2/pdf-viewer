# 1427 — A rank over a population that grows is written as a comparison with a named member, and an ADR's appendix is not its decision

Session 1295. Status: accepted. Context: `CLAUDE.md` *Where knowledge lives*; the habit in
`doc/habits/the-ledger-and-claims-about-this-tree.md` that an unread claim is edited rather than
corrected by a sentence appended after it; ADR 1355 (the overtaken sweep's read list); ADR 1415 (a
log of moves moved whole into an appendix). Code: `tools/superlatives.py`, `tools/state.sh`
(`section_superlatives`), `tools/conformance/src/overtaken.rs` (`decided`, `READ`).

## 1. A superlative is a claim about members nobody has written yet

`oracle.rs` said pages 2 to 5 of `issue12963.pdf` agree "to 0.0004 of 255 — the tightest limit this
bucket has measured", and page 1 of the same document now agrees to 0.00003. A superlative scoped to
a population that grows, such as the tightest in this bucket, the largest fall this list has had or
the smallest rasters in the bucket, is true when written and made false by a row added later. Nothing
points back at the sentence from that row. The same file held four such sentences about one
quantity, each written as the record, and three of them already contradicted one another.

**Decision.** `tools/superlatives.py` (and `tools/state.sh superlatives`, a count) lists every
sentence in the corpus gates' doc comments, `raster_golden`'s notes and the ledger's notes that
pairs a rank word with a scope over a growing population, and prints the numbers the sentence
asserts. It is a reading list and never a gate, because whether "first" means first in time or
first in rank is a question about English. An overtaken sentence is rewritten as a comparison with a
**named** member ("twice as tight as `issue12963.pdf` pages 2 to 5's 0.0004"), which a later row
cannot make false. Otherwise the rank is dropped or deferred to the command that prints the ranking.
A rank that is still true of its scope is left alone, and so is an ordinal in time ("the first page
to...").

## 2. An appendix carries its own dates

The overtaken sweep read each ADR's full text as a decision taken at the ADR's number. ADR 1415's
appendix is `corpus.rs`'s log of moves, moved verbatim. So one record named a page of six lists no
later decision had touched, and it was the only overtaking decision for each of them. **Decision:**
`overtaken::decided` reads an ADR above its `## Appendix` heading only. Of the 37 notes that sweep
listed, six were that record alone. The other 31 were read against the decisions the sweep printed
and entered in `READ`. Eight had a sentence made false, and those sentences were rewritten as what
is, each named in round 1295's record.

## Cost

A decision that argues about a page inside an appendix is no longer seen by the sweep. No ADR does
so today: 1415's appendix is the only one. A rank rewritten as a named comparison says less than the
rank did. Where the rank mattered, the command that ranks is named instead.

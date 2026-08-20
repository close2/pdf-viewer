# 0455 — The status a clause never asked for

Status: accepted.
Session: 620. Follows ADR 0452, whose finding was that `doc/todo/01`'s blame ordering is not
exhausted below commit 534; this is the band it named, read.

## The decision

**§7.9.2 moves from `partial` to `implemented`, and the reason it was `partial` becomes an
observation rather than a status.**

Its note had said, since the ninety-sixth commit of the tree:

> What is not here is the *typing*: the object model carries one string type and each reader
> decides which of the four it holds, so a text string read as bytes is a defect nothing in this
> crate can catch.

That is a true statement about this tree. It is not a statement about ISO 32000-2 §7.9.2, and
`partial`'s own definition in `ledger.toml`'s header is about the standard:

> partial       some are; the note says which, which are not, and what is reported

— *some* of the clause's normative requirements are executed. Which requirement of §7.9.2 is not?
The clause's answer is that a reader supplying the qualification per entry is exactly what it
describes:

> PDF supports one fundamental string object (see 7.3.4, "String objects"). The string object
> shall be further qualified as a text string, ASCII string, or byte string. The further
> qualification reflects the encoding used to represent the characters or glyphs described by the
> string.

**The ledger already contained that reading and it was one row away.** §7.9.2.1 is `implemented`
and its note says "[t]his crate holds exactly that — one `Object::String` of bytes, qualified by
whoever reads it." So the family carried two opposite readings of one sentence: the parent called
the single string type the debt, the child called it the match. The child is right, on the
clause's own first six words.

## Why this took five hundred rounds to notice

**The sixth sweep has printed this row every time it has run.** `doc/todo/01` records it under
"Arithmetic (sweep 6)" in every one of the sweep blocks since the three-hundred-and-seventy-fifth
session, always in the same words: "two hits, §7.9.2 and §O, read and kept before. Clean."

The sweep was right and the dismissal was wrong, and the mechanism is worth naming because it is
general. Sweep 6 asks *which parents are behind their children* — an arithmetic question with an
answer in the file. What "read and kept" then did was answer it with a **sentence**, written once
in the three-hundred-and-forty-second session, that a dozen later rounds cited instead of
re-deriving. A sentence recording that a row was read is evidence about a round; it is not evidence
about a row, and it does not decay in any visible way when the row it describes turns out to have
been read badly.

That is ADR 0452's finding restated from the other end. 0452 found that a *band* was read as a
*floor*. This finds that a *reading* was read as a *verdict*. Both are the same substitution: a
fact about what a session did, standing in for a fact about the tree.

## The rule this leaves

**A dismissal of a sweep hit must re-derive, not cite.** A hit answered with "read and kept in the
Nth" is answered with a fact about session N. The three things that make a dismissal checkable are
the ones the five-hundred-and-sixty-fifth session's `§8.7.4.1` re-derivation used and the ones
these notes now carry:

- the clause's own sentence, quoted, so a later round can see what was read;
- the code that meets it, named, so the claim has an address;
- the population, measured, so "no document does this" is a number rather than a memory.

`doc/todo/01`'s reading-order section gains this as its rule for *choosing*, which is the thing
the round was asked for and which neither the sweeps nor the blame order supplies on its own:
**rank by blame, but read the row whose stated reason is a claim about this codebase rather than
about the standard.** Every defect this band produced was of that shape — §7.9.2's architecture
preference, §8.10.4.3's count of a list it had not recounted, §7.6.5.3's names for two algorithms
it had run together, §7.6.4.4.2's cited test. A note that quotes the clause is a note somebody
checked against the clause; a note that describes the tree is a note that was true when it was
written and has been ageing ever since.

## What was not decided

**§8.10.4.1 stays `partial`, and the argument for moving it was made and rejected.** Its note
compares itself to §10.7.2, which is `implemented` because "[t]he normative content is a
permission, and it is exercised" — and §8.10.4.1 does contain such a permission, exercised
literally, since nothing in `crates/`, `tools/` or `fuzz/` names `Ref` at all:

> PDF processors that do not recognise the Ref entry shall simply display or print the proxy as an
> ordinary form XObject.

The difference is what surrounds it. §10.7.2's *whole* normative content is the permission.
§8.10.4.1 grants it in one sentence and then spends the rest of itself on the processors that do
import — Table 95's two required entries, the proxy's `Matrix` and `BBox` becoming the imported
page's bounding box, the `Group` that carries over. Those are requirements, they are not executed,
and a clause that says what a processor which *does* implement the feature shall do is not
discharged by declining the feature. `partial` is what that is.

**No report was added for a reference XObject**, and the reason is measurement rather than
judgement: `witness_census` over all 1251 PDFs on this disk finds two documents stating `/Ref` as a
name and both are §14.7.2 Table 355's structure-element `/Ref`, not Table 93's. There is no
population to report to, and the clause does not ask for a report — it asks for the proxy, which is
drawn.

## Cost

One row's status changed and one unit test added. The status change is visible to
`cargo run -p conformance --bin ledger`'s totals and to nothing else; the test is the first thing
in the tree that reaches §7.6.4.4.2's steps (a) to (d) at all, which is the other half of this
round and is in `doc/history/620`.

# 0319 — A sweep that has to be rebuilt, and the entry behind a capability

**Status.** Accepted.
**Context.** `doc/todo/01` describes fifteen sweeps. Fourteen read what a ledger row *says*; the
fifteenth reads no reason at all and asks instead who reads the entries the clause states, which is
the only one that can see `doc/habits.md`'s sixth refusal shape — a row that retires its refusal by
naming a capability that arrived. It was built in the four-hundred-and-sixtieth session (ADR 0295)
and produced §12.5.6.15's unreachable attachment; it was run again in the four-hundred-and-eightieth
(ADR 0315) and produced §12.5.6.2's nine group entries. **Neither run left a program behind.** The
second round found the sweep existed only as thirty lines of prose and rebuilt it from that
description before it could run it at all.

## The decision

**The sweep is `conformance::entries` and `cargo run --release -p conformance --bin entries`.**

`CLAUDE.md`'s rule is that a fact which can be counted is not written down and *the command that
counts it* is. This file had the description and not the command, and the failure it produced is
exactly the one the rule predicts: a round that wants the sweep pays to reconstruct it, and **a
reconstruction is not the same instrument twice**. The three runs' numbers are not comparable and
this ADR says so rather than tabulating them beside each other.

It is a program in `tools/conformance` rather than a script beside the tree for the reason the
quotation sweep is (ADR 0309): the ledger's parser, the standard's clause index and the source walk
are already there, tested, and dependency-free. What is added is `entries.rs` and a binary.

**It is not a gate**, on ADR 0249's ratio argument. Most hits are refusals `CLAUDE.md` already
closes and rows whose `code` array has gone stale beside a crate that grew a second file. What
decides whether a hit is work is a question no program can ask.

## What the program does that the description did not

The second run ended with a lesson it could not act on: a hit is work only where the row's disposal
of the entry is a claim about *the entry* rather than about *the clause* — "not a rendering
question" was the first kind and was wrong; "reaches a comments pane" was the second kind, was right
about the entry, and was silent about the paragraph the entry points at.

A program cannot tell those apart. It can tell a third thing from both, and that is the one added
here: **whether the row's own note names the entry at all.** A note that never writes the key has
made no claim about it, which is the shortest reading list the sweep can produce. Its first run as a
program: 215 rows in the population, 41 stating an entry their own `code` does not name, 102
entries — 30 named nowhere in the tree and 72 only elsewhere — of which **43 are not named by the
row's note either**.

Two other choices, each because the reconstruction had to make one and nothing recorded it:

- **A clause's tables come from its *own* text**, up to the next numbered heading, and not from
  `Heading::span`, which runs to the end of the clause's descendants. The span is right for a
  quotation and wrong here: §12.5's row would own every annotation table in the standard.
- **An entry counts as named in two forms**, `"Key"` and `/Key`, and never as a bare word. `Name`,
  `Type` and `Border` occur in a hundred sentences that have nothing to do with the clause, and a
  sweep whose hits are noise is a sweep nobody reads.

## What its first run as a program found: §12.6.4.2's `/SD`

Table 202 gives a go-to action two destinations and states which wins:

> (Optional; PDF 2.0) The structure destination to jump to (see 12.3.2.3, "Structure
> destinations"). If present, the structure destination should take precedence over destination in
> the D entry.

`grep '"SD"' crates/` found **nothing**, and §12.3.2.3's own ledger row has read `implemented` since
the algorithm that walks a structure element's `/K` back to a page landed — with two tests, one for
the recursion and one for the clause's fallback to page one. So the capability was built, tested and
cited, and **the entry that turns it on was wired to nothing**. Every structure destination this
program had ever resolved arrived because a link's or an outline item's `/Dest` array happened to
begin with a structure element, which is a form no document on this disk uses.

The row said otherwise, and the sentence is the shape's own signature:

> `/SD`, the structure destination alternative, resolves through §12.3.2.3 like any other.

True of `Destination::read`, which resolves a structure destination handed to it. False of this
clause, which never handed it one.

**Third instance in twenty-five rounds** (0295, 0315, this), and the sharpest of the three, because
the other two were entries with no reader anywhere. This one has a reader, a clause, a status of
`implemented` on both rows, and a sentence in the ledger asserting the connection that was missing.

## What was built

`Destination::of_go_to` reads Table 202's two entries in the table's own order, and both callers of
a go-to action's destination use it: `action::one`'s `GoTo` arm and `Destination::open_action`.
Nothing else in the standard states an `/SD` — Table 203's is a remote go-to's, whose first element
is "a byte string representing a structure element ID in the remote document" and which
`CLAUDE.md` excludes with the action.

**An `/SD` this reader cannot read as a destination falls back to `/D` rather than refusing**, and
that is a reading rather than a convenience: the precedence is a `should`, `/D` is the *required*
entry, and jumping to it is conforming. An `/SD` that is a destination always resolves, because
§12.3.2.3 ends its own algorithm with "the page reference shall be assumed to be the first page in
the document" — so the fallback fires only on a malformed entry.

## Counted before believed

`crates/pdf-model/examples/structure_destination_census.rs`, over every object the cross-reference
table lists — because a go-to action hangs off a link, an outline item, an `/OpenAction`, a widget's
`/AA` and another action's `/Next`, and a census visiting only one of those measures the walk.

| | |
|---|---|
| documents opened | 1187 (the 974, the four submodule corpora, `doc/`) |
| `/S /GoTo` action dictionaries | 7838 |
| stating Table 202's `/SD` | **10**, in 2 documents (`bug1997343.pdf` 8, `bug2009627.pdf` 2) |
| of those, readable as a destination | 10 |
| naming a page their `/D` does not | **0** |
| naming a different *view* of the same page | **0** |
| `/S /GoToR` stating an `/SD` | 0 |

**So the corpus cannot rank this sentence**, and the fixture is trap 8's prescription: three go-to
actions differing in the single entry, one with `/D` alone, one with an `/SD` that names another
page *and* another view, one with an `/SD` that is no destination at all. Every one of the three
assertions was watched fail with its rule removed.

This is the same shape ADR 0315 recorded for `/IRT` — an entry ten documents state and none of them
exercises. It is worth saying once more that ten witnesses of an *entry* and zero of a *rule* are
different measurements, and only the second ranks work.

## What was not done

- **No report.** An `/SD` that reads as no destination is a malformed entry the clause gives a
  conforming answer for, and a report would take a page off the oracle's judged set for a jump that
  goes where the required entry says (trap 11).
- **Table 203's `/SD` stays unread**, and §12.6.4.3's row says so now rather than leaving the
  neighbour lying — the seventh failure shape's cure, applied in the same commit.
- **The other 42 undisposed entries were read and not taken.** They are refusals `CLAUDE.md`
  already closes (a sound action's four, an ECMAScript action's `/DS`, a seed value dictionary's),
  entries a `reported` row's own refusal covers, and two artefacts of `doc/md/` shifting a table's
  columns — Table 165's `/name`, which the four-hundred-and-eighty-first met from the other side,
  and Table 200's, which the four-hundred-and-sixtieth did.

## Consequences

- One sweep of the fifteen is now a command. The other fourteen are still descriptions, and the
  argument above applies to every one of them: the next round that rebuilds one should commit it.
- `doc/todo/02` §4 names the invocation, so a round does not have to read `doc/todo/01` to run it.
- §12.6.4.2 and §12.3.2.3 each record the other, which is the seventh failure shape's remedy: a
  mechanism gets one row per clause that mentions it, and the rows are written in different
  sessions by different reasoning.

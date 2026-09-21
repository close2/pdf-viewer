# ADR 1166 — The documents that came after a departure

Status: accepted, 2026-09-21.

The instrument half of the owner's note on ADR 1119. ADR 1165 is the sweep it was built for.

## What ADR 1119 could not take, and why the gap was taken by design

ADR 1119 gave `departed` its one check: a row wearing the word must name an ADR, because "decided
against with its cost recorded" is a claim about a document somebody can open. The ADR is explicit
that the check "refuses a row naming no argument, and cannot judge whether the argument fits", and
that shallowness is right — a rule that scored a premise would be a judgement wearing a program, the
same reason `Ledger::is_aggregate` reads no prose (ADR 1035 section 4).

What the shallowness leaves is the cost `doc/habits/the-ledger-and-claims-about-this-tree.md` names:
a capability recorded as blocked on a decision outlives the decision, and **nothing fires when a
stated blocker expires**. A `departed` row owes nothing, so no sweep over the debt reaches it, and
its argument ages out of sight. ADR 1165 found one of twelve in exactly that state and needed a day
of hand-reading to find it.

## The decision

**Two things, and the split between them is the point.**

### 1. The argument has to be a document that exists — checked, in the gate

`Problem::ArgumentMissing` fails `cargo test -p conformance` where a `departed` row's note names an
ADR number with no file in `doc/adr/`. This is ADR 1119's check one step later: a number that names
no file settles a row on prose exactly as naming no number does, and the difference is invisible to a
reader who never types the path.

It reads a directory listing and nothing else. `Problem::ArgumentsUnreadable` fires where the
directory cannot be listed, because a sweep that could not open its population and said nothing has
printed a tick about itself (trap 13); and nothing is read at all unless some row is `departed`, so
a ledger without one pays no listing.

### 2. The reading list is printed, and judged by a person

`cargo run -q -p conformance --bin departures`, and `tools/state.sh departures`. For each `departed`
row it prints the ADR the note's **first sentence** names — the deciding argument — every other ADR
the note names, and every **later** ADR that cites that argument or the row's own clause number.
That is the set of documents somebody has to read to find out whether a premise still holds. Whether
it holds is not printed, not scored and not guessed: no program can read a premise, and one that
pretended to would be believed.

## Three readings inside it, each with what it costs

- **The deciding ADR is the one in the first sentence.** A `departed` note opens with the departure
  and its argument by the convention the status was created with; the rest of the note is the
  clause's other requirements, and the longest accumulates twenty-three ADR numbers. A reading list
  built from all of them is one nobody reads, which is how an instrument stops being run. The rest
  are printed on their own line, so nothing is hidden — they are just not the argument.
- **A sentence ends at a full stop followed by white space.** A clause number's full stops are
  followed by digits, so `§12.11.3` does not end a sentence. That is the whole of the rule and the
  whole of its cost.
- **Later means later than the last deciding ADR.** A re-reader wants what came after the argument
  was complete. An ADR citing the clause before the decision was taken is part of the argument's own
  history rather than of what has happened to it since, and `doc/history/` is where that is kept.

The citation matcher takes the three forms this tree writes — `ADR 0036`, `ADRs 0803, 0814, 1144,
1145` and `ADRs 0718/0725/0737` — and stops at the first thing that is neither a separator nor a
four-digit number, so `ADR 1035 section 3` names one ADR. A clause citation matches the clause and
not a subclause of it: `§12.5.5` in a document about `§12.5.5.1` is a different subject, and a
reading list that mixed them would be longer and worth less.

## Why this is not a ratchet, and not a failure

A premise that has expired is a question for a person, and the honest answer to it is sometimes that
the row is right anyway — eleven of ADR 1165's twelve were. A gate that failed on the *age* of an
argument would fire on every run and stop being a signal (trap 39). So the count is printed beside
the list and the only thing that fails is the one mechanical claim: the argument is a document that
exists.

## What it does not do

It does not read the ADR's prose, classify it, or compare it with the tree. The four shapes of
expiry ADR 1165 sweeps for — a built capability, the output device, an amended exclusion, an
inconsistency — are stated in the program's closing paragraph as what to look for, and looked for by
whoever runs it. That is the same division `tools/conformance` has held since it was written: the
checker establishes that a claim is well formed, and only reading establishes that it is true.

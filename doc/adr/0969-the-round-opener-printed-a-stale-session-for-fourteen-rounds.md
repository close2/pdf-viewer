# 0969 — The round opener printed a stale session for fourteen rounds, and its fifth-round signal was always yes

Status: accepted. Session 963.
Context: `tools/round.sh`, `doc/history/`, `doc/todo/02-every-round.md` §2,
`doc/traps/instruments-and-reports.md`. It **amends ADR 0281**'s account of where a counted fact
comes from, by correcting the one number `tools/round.sh` computes.

## What happened

`tools/round.sh` printed `session 945` on this tree for fourteen consecutive rounds, while
`doc/history/` grew files 946 through 959 and `doc/history/945-a-gate-that-was-not-running.md` sat
there describing something else entirely. Two ADRs written in session 959 carry `Session 945` on
their status line because the round that wrote them asked the opener and believed it.

## Why, and why the reason was a good one

The script computed the session two ways and preferred the wrong one, on an argument that was
right when it was written:

> **A parallel round's branch outranks `doc/history/`, and that is not a preference.** A worktree
> is branched before its neighbours have written their files, so `ls doc/history/` there is a count
> of the rounds that finished *before this batch started* — [it] told the six-hundred-and-eighty-seventh
> and the six-hundred-and-ninety-first that they were session 685 […]. The branch name is the
> assignment itself and cannot go stale.

Every sentence of that is true of the situation it describes. The last one is a **claim about the
world**, and the world changed underneath it: this project stopped branching one branch per round
and started running long campaigns on one — `round-940/pdf-a-validator`, then
`round-945/the-fifth-round`, which carried sessions 946 to 959. From the second round on that
branch, the branch name was the *staler* of the two sources.

So the defect is not a bug in the arithmetic. It is a **preference between two sources, hard-coded
in the direction that was correct for the workflow of the day** — which is the same shape as a
ledger row whose reason has outlived the capability it names, and it decayed the same way.

## The half that is worse than the wrong number

945 divides by five. `doc/todo/02-every-round.md` §2 makes every fifth round run the full gate
sequence whatever it touched, *because a change→gate map is a claim about the crate graph and a
claim decays* — and `tools/round.sh` is what says which round that is. For fourteen rounds it said
**every** round was a fifth round.

Nothing broke, and that is the point worth recording. The failure was in the safe direction: the
full sequence ran when only the core four were owed, which costs minutes and hides nothing. **An
always-yes signal is not a conservative failure — it is a dead instrument.** The fifth-round rule
exists to make a periodic, unconditional check distinguishable from a targeted one, and a signal
that fires every time cannot make that distinction. Fourteen rounds have no record of whether they
were fifth rounds, and the three that actually were (950, 955, 959 by `doc/history/`'s count) are
indistinguishable from the eleven that were not.

This belongs beside trap 5: an instrument that always answers the same way has stopped being an
instrument, whichever way that is. A gate that always passes is the familiar shape; a gate that
always fires is the same defect wearing the reassuring half.

## The decision

**Neither source outranks the other. The session is the later of the two.**

Each is a floor, and each goes stale in its own direction:

| source | too low when |
|---|---|
| `ls doc/history/` + 1 | a parallel worktree branched before its neighbours wrote their files |
| the branch's own number | the branch has carried more than one round |

`max` is correct in both cases and stale in neither. A worktree branched as `round-960/…` takes 960
over a `doc/history/` that stops at 954, which is exactly what ADR 0281's predecessor argument
asked for. The fourteenth round on `round-945/…` takes `doc/history/` over the branch. Nobody has
to remember which source to trust, which is the property that makes this the fix rather than
renaming the branch.

The script also now *says which source won*, and when `doc/history/` wins it prints how many rounds
the branch name has carried — so a campaign that has outgrown its branch name is visible in the one
place every round already looks.

## What was not done

**The two ADRs are corrected in place rather than left standing.** `doc/history/` is a record and is
written once; an ADR's status line is not a record of a round, it is a pointer to which round argued
the decision, and a pointer to a session that describes a different piece of work is simply wrong.
Each now names the session it belongs to and says what it said when written, which keeps the
correction honest without rewriting either argument.

**The branch is not renamed.** It has fourteen rounds of commits on it and the name is now accurate
about nothing except when the campaign started, which is a fine thing for a branch name to be.

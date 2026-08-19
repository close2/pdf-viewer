# The history directory — one file per round

Status: **record** — written once, never rewritten.
Read by: whoever wants to know when something landed and which ADR argues it. **No round reads
this directory to do its work**, which is the property that makes it the right place for
bookkeeping.

**A round writes one file here and nothing else about itself.** Name it
`<session>-<slug>.md`, so that `ls` sorts by session:

```
doc/history/446-a-fact-that-can-be-counted-is-not-written-down.md
```

A file opens with the session number and its one-sentence finding, then the date, the ADR that
argues it, and the files it touched. What follows is prose — as much or as little as the round
earned.

## Why a directory rather than another row

`doc/history.md` is a table with one row per session and it holds 5 to 445. It works, and it is
kept exactly as it is. **The one thing still written there is a *block* summary** — what a run of
twenty or thirty rounds had in common — which a closing round appends below the table beside the
others, because it is about a run of rounds rather than about one and the per-round files already
hold each. That is the only exception to "a round writes one file here and nothing else about
itself", and it belongs to the closing round alone. What the table cannot do is take more than a
sentence without becoming unreadable, and that is the pressure that produced this round's whole problem: a round with more to
say than a cell holds says it somewhere a round *reads*, and the gate table in `doc/HANDOVER.md`
grew 816 lines of per-round narrative that way.

Three things a file gives that a row does not:

- **No shared line.** Two rounds appending files never conflict; two rounds appending rows to one
  table do, and resolving that conflict means editing somebody else's sentence.
- **Room, without cost to anybody.** A round that found something worth five paragraphs writes
  five paragraphs, and no round that is not asking about session 446 ever loads them. A row that
  wanted five paragraphs used to end up in an instruction file.
- **A round's own record is one write.** `doc/todo/02-every-round.md` §6 asks for exactly one new
  file, which is checkable — and a round that edited a *neighbouring* session's file would be
  visibly doing something it was not asked to.

Sessions 5–445 are not split up. Splitting them would rewrite a record for tidiness, which is the
one thing a record may not have done to it.

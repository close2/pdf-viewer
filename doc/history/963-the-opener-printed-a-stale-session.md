# 963 — The round opener printed a stale session, and its fifth-round signal was always yes

Date: 2026-09-11. ADR: 0969.
Files: `tools/round.sh`, `doc/adr/0961-*.md` and `doc/adr/0962-*.md` (status lines),
`doc/history/955`–`959` (the record this round was written to catch up).

Found while writing the owed `doc/history/` files for sessions 955 to 959 — which is the only
reason it was found at all, because writing a record is the one job that reads the session number
*and* the directory it is derived from in the same minute.

`tools/round.sh` had printed `session 945` for fourteen rounds. Two ADRs from session 959 say
`Session 945` on their status line, next to a `doc/history/945-` file about a gate that was not
running. The script preferred the branch name over `ls doc/history/` on an argument that was
correct for one-round branches and went stale when this project started running campaigns on one
branch. **Neither source outranks the other; the session is the later of the two**, and the script
now says which one won.

The worse half is the fifth-round signal. 945 divides by five, so `doc/todo/02-every-round.md` §2's
periodic full sequence fired **every** round for fourteen rounds. Nothing broke — it over-ran the
gates, which costs minutes and hides nothing — and that is why it survived. **An always-yes signal
is not a conservative failure, it is a dead instrument**: the rule exists to make an unconditional
periodic check distinguishable from a targeted one, and a signal that fires every time cannot make
the distinction. Fourteen rounds have no record of which kind they were.

The general shape, which is why this got an ADR rather than a one-line fix: a hard-coded preference
between two sources of a fact is a **claim about the workflow**, and it decays exactly the way a
ledger row's reason does. The comment that had to be deleted said the branch name *cannot go
stale*.

---

**And a second instrument of the same shape, found by the viewer stream and argued here** (ADR
0970). `accessibility_census`'s ceiling on *pages with no `/StructParents` whose fallback answered
nothing* was 56 and the run found 60 — four pages of a profile specification downloaded into `doc/`
the day before. The four pages are not the finding. `population()` is a `read_dir` of `doc/`,
`.gitignore` excludes the specifications this project buys, and **sixteen of the sixty entries are
such files**: the ceiling had been silently re-baselined by downloads for as long as it has existed,
and only failed now because one finally pushed it over instead of being absorbed.

The guard in the same function is the instructive part. It skips the **floors** when the population
is smaller than the one they were measured over, with a correct argument — and that argument is half
of itself. A floor is wrong when the population shrinks; a ceiling is wrong when it grows. It was
written by somebody thinking about a missing submodule, and it covers only that direction.

Held by name now, which matches the population's own asymmetry: a page that *joins* the class fails
the run and is named, a name that is merely *absent* fails nothing — so a fresh clone, which has
none of `doc/`'s specifications, passes without the gate being weakened for anybody. A count cannot
say that, because it cannot tell a page that left from a page that was never there.

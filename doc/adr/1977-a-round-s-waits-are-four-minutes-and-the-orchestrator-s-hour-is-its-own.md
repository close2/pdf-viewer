# 1977 — A round's waits are four minutes, and the orchestrator's hour is its own

Session 1570, slot 7 of batch ninety. Status: **accepted**. Context: ADR 1976 (the census, the price
and `tools/wait.sh`). Text: `doc/todo/02` section 0 item 3, the rule block that opens
`doc/environment.md`, `doc/todo/_brief-template.md`'s section 1, the line `tools/round.sh` prints
under "the round", and habit 104 in `doc/habits/measuring.md`. It cites no clause and moves no row.

## 1. The rule, and where it is written

**A command that can run past 240 s is started with `tools/wait.sh --start <log> -- <command>` and
waited for with `tools/wait.sh <log>`, in calls whose harness timeout is 270 000 ms, re-issued on exit
75; no wait-loop of a round's own runs longer than 240 s.** The heavy-walk rule's "in the foreground,
and you wait in it" kept its reason — the round does nothing heavy beside its walk, and it does not
end its turn while the walk runs — and lost its form: the wait is the round's, the foreground call is
not. A locked run started this way holds its lock in the detached session exactly as it did in the
call, so the lanes, the log line and `HEAVY_WALK_HELD_BY` are unchanged.

The rule is in the four places a round reads before it runs anything, once each: section 0 item 3
says it as tier 1's way of waiting, the environment rule block amends the heavy-walk line, the brief
template's section 1 names it so a brief points at it rather than restating it, and `tools/round.sh`
prints it beside the tiers. Habit 104 carries the incident.

## 2. Why it stops at the rounds

The orchestrator's transcript is written at the one-hour lifetime (24.1 M tokens, all of them at the
hour) on Claude Fable 5.1, whose hits cost 0.025× and whose writes 2×. `tools/cache-census.py --ttl
3600 --write 2 --read 0.025` over it: 3 176 requests, 36 of them after a gap over an hour, writing
18.1 M tokens — 75 % of its writing — and **35 of the 36 followed no command at all**: they are turn
ends with a mean gap of 45 269 s, the waits between batches for the owner and for the quota, in which
no request can be sent by anybody. Its waits on rounds sit inside the hour. So a four-minute wall
there would buy nothing and cost a request every four minutes of every batch; the orchestrator keeps
its own hour, and the rule binds a round.

## 3. What is left open

Whether the saving arrived is the next batch's count: `tools/cache-census.py --since <its start>`
over its round transcripts, the share of writing after gaps over 300 s against this ADR's 85 %. A
round that waits through the harness's own background task (`run_in_background`) waits on its output
file with `tools/wait.sh --until '<its end line>' <file>`; no gate holds a round to the rule, since
the only witness of a wait's length is the transcript, which the tree does not hold.

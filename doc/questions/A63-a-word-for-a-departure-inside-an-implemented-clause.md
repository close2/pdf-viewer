Status: complete
Given: 2026-09-14, in conversation — transcribed by the round
Owes: the `departed` status — `ledger.rs`'s enum, the aggregate rule, `tools/state.sh`'s counts, `ledger.toml`'s header, and the twenty rows re-statused (acting round)

> Add `departed`.

Reading: the word means *every requirement of this clause is executed except the one the note
names, which was decided against with its cost recorded*. Two riders come with it, and both are
what keeps it from being the hiding ADR 1035 refused:

- **`tools/state.sh` gains a `departed` count.** Departures stay visible as their own figure —
  the new word separates the decided sentence from the debt count; it does not erase it.
- **The aggregate rule is ADR 1035 §5's**: a `departed` row owes nothing, and a parent above it
  owes nothing on its account.

The twenty rows stop wearing `partial`'s word for unfinished work, every "how much is left"
figure stops being twenty too high permanently, and a round handed one of them reads the note
instead of spending its slot discovering a session-400 decision.
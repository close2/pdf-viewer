# 1748 — A premise is a command and its output, a hypothesis is briefed as one, and a review keeps its scripts

Session 1456. Status: accepted. Prose: `doc/reviews/1456-ten-batches-on-the-template.md` (the
measurements), `doc/todo/_brief-template.md` (the slot's part), `doc/todo/02` section 0 item 2.
Scripts: `doc/reviews/1456/`. Amends ADR 1638 decision 3 and review 1401 section 4 rule 10; every
other rule of that section is kept, each by its figure (the review's section 3).

## 1. The question

Review 1401's rule 10 asked for its figures to be re-taken ten batches on. Re-taken over batches
sixty-three to seventy-one, eight of its ten rules hold by their own numbers. Two do not:

- **Rule 3, "check every contract's premise before writing it".** 47 of the 54 records now state a
  premise, against 10 of 49, and 29 of those 47 say that part of it did not hold, against 4 of 10.
  Batch by batch there is no trend. By class there are 5 figures, 3 places, 7 mechanisms (where
  the time goes, what a digest follows) and 14 claims about how far a feature reaches. The rule
  made the premise visible without making it right. And the mechanism class can never be checked
  by an orchestrator's grep, because a round finds it by profiling (habit 59).
- **Rule 10 itself.** Review 1401's scripts lived under its worktree's `scratchpad/` and went with
  it, so this re-take had to rebuild each one from a prose description. The tree's own figure did
  not fill the gap either: `tools/state.sh batches` sums only the last digit group of "7 497 s"
  and finds no gate line, so every batch body from sixty-four on is mis-summed.

## 2. Decisions

### 2.1 The slot's premise line is split in two

- **Premise and its evidence** carries only what the orchestrator checked, each claim given as the
  command run and what it printed, with the unit: "`grep -rn X crates` gives two lines, both
  tests". A figure and a place, 8 of the 29 misses, are of this kind. A command finds them in
  seconds before the brief, so a premise without its command is not briefed.
- **Hypothesis** carries what the orchestrator believes and has not checked, or cannot check by a
  command: where a cost lies, what a gate's figure follows, how far a feature reaches in a window.
  The contract says what the round builds if it is false. The round tests it first, by profiling
  or by reading the site, and reports the result the same way as a premise. The slot keeps its
  25-line cap, and the line is left out when there is nothing to put in it.

The cost is one line in a slot that uses it, and an orchestrator who must say which kind each
claim is. Part of the gain is honesty in the brief. The other part is a round that starts by
testing its hypothesis instead of arguing with its brief. Of the 29 misses, 21 were of the two
kinds this line now names.

### 2.2 A review's scripts are committed beside it

A review that re-takes figures commits the scripts it counted with under `doc/reviews/<session>/`,
and names each command at the table it fills. `doc/reviews/` is a record, so the scripts are never
edited. A later re-take that needs a change copies a script into its own directory and says what it
changed. The scripts are read-only over git and `/home/AI/`, so a later re-take can run them with
no lock.

## 3. Declined

- **Moving the batch's arms export ahead of the launch.** It holds lane 1 for 1 694–2 474 s at
  `open`, and three batches' longest round wait was behind it. Moving it first would delay all six
  rounds by that much, where it now delays only those whose first walk comes early.
- **A record shorter than forty lines, or one cut to a table.** 33 of the 54 records sit at 39 or
  40 lines, and the cap is the target. But no round reads a record, and review 1401's reason for
  declining a table still holds. This is recorded as a cost and left alone.

## 4. Owed, and to whom

- `tools/state.sh batches` reads a figure with a digit group and a gate line written that way.
  Until it does, `doc/reviews/1456/slots.py` is the reading. This is owed to whichever slot holds
  `tools/state.sh`, which in this batch is slot 5.
- The navigational re-read that ADR 1638 decision 7 puts every fourth batch ran at batch sixty-six
  (round 1423) and was not briefed at batch seventy. It is owed to the next brief with an
  instruments slot.
- The next re-take is due ten batches on, at batch eighty-one or eighty-two. It runs
  `doc/reviews/1456/`'s commands, listed in the review's section 4.

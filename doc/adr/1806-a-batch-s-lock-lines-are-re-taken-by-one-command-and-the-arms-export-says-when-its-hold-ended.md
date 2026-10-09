# 1806 — A batch's lock lines are re-taken by one command, and the arms export says when its hold ended

Session 1485. Status: **accepted** and **built**. Answers the re-asks of ADR 1756 section 5 and ADR
1790 section 6 on batch seventy-five's lines, and gives both one command. Amends ADR 1650's README
(a last line). Context: ADRs 1650, 1662, 1706, 1756, 1790; traps 13, 25, 130.
Code: `tools/state.sh` (`lock_cost`: `LOCK_COST_BATCH`, the pairs out of ask order, the re-take
line), `tools/batch.sh` (`export_arms`, `arms_released`). Tests: `tools/conformance/tests/bounded.rs`
(`the_lock_cost_names_a_walk_granted_before_one_that_asked_first`), `tools/conformance/tests/batch.rs`
(`arms_says_when_its_hold_of_the_lock_ended_in_the_logs_own_words`). Instrument:
`/home/AI/lock-replay-1485/` (`retake.py`; ADR 1706's `replay.py`, which now records the lane each
replayed grant took; `log67.py`, which reads `HEAVY_WALK_LOG`; `table74.py`, unchanged).

## 1. Batch seventy-five's lines

`grep -c 'batch=batch-1474-1479' /home/AI/heavy-walk.log` prints 50: 13 rounds' lines, the arms
export, 3 `check` runs and the merge's 33. The replay takes the 17 that are not the merge's: 15 walks,
**no clock run**, one long hold, the export.

- **ADR 1756 section 5.** The long hold was round 1478's nesting census, `--long`, killed at its lane's
  6 GiB (peak 6.77 GiB, exit 137) after 828.5 s; it ran again as a large walk, 803.5 s on the first
  lane after 844.8 s of queue. Two small walks asked inside it and queued 998.6 s, behind it and the
  export together. No clock run was asked inside a long hold: C = 0.0 s, and the turn saved the walks
  S = 0.0 s. **The turn stands, untested** — as on batch seventy-four's lines (round 1472).
- **ADR 1790 section 6.** One pair out of ask order: round 1474's `raster_golden`, asked 01:56:45,
  was granted the first lane at 02:02:48, while round 1475's walk, asked 01:53:41, waited to
  02:04:16. Both wrappers had started before ADR 1790's was renamed in after the export's `done`
  (02:02), so it is the race and not a defect; no pair falls after it. The walks queued 2 774.6 s
  observed and 2 773.8 s in the race replay (17 of 17 lines within 1 s, the walks' summed error
  2.8 s): **the order stands**. Ask order replays to 2 802.6 s, max 1 018.0 s: it swaps the pair
  (998.6 → 940.4 s for the two), and ADR 1706's chain then asks 1475's drive 88 s earlier behind the
  same release, so it waits 1 018.0 s where it waited 930.7 s and is granted in the same second. The
  28.8 s is the chain moving an ask, not a later grant.
- **The longest queue was not behind the batch's export.** 1475's drive queued 930.7 s behind round
  1478's own six-arm run (`scratchpad/r1478/arms.sh`, 1 549.2 s on the first lane), which compared the
  round's change against the export; the export's 2 180.8 s held two queues of 363.0 and 635.6 s.

## 2. The command

`tools/state.sh gates-cost` reads the batch `LOCK_COST_BATCH` names, or the last, and its last lines
are now (a) every pair of walks whose asks are whole seconds apart where the later was granted, more
than a second before the earlier, a lane the earlier's ticket takes — a small walk's both, a large
one's the first, a long one's the second; clock runs take no ticket and are not read — and (b) the
re-take, `cd /home/AI/lock-replay-1485 && PYTHONDONTWRITEBYTECODE=1 python3 retake.py <batch>`, with
the batch filled in. A re-take is asked after the next batch's first line has landed, which is why
the variable exists. `retake.py` prints the population by origin, section 5's C, S and verdict, and
section 6's pairs and the observed queue against the race replay within the replay's own summed
error on the same lines; and the rules' table `table74.py` printed.

**Calibrated** (trap 13): the two implementations name the same pairs on batch seventy-four's lines
(11, among them ADR 1790's incident) and seventy-five's (1); on the ask-order replay's own grants of
batches seventy-two to seventy-five they name none, on the race replay's 2, 0, 10 and 1. With the
lane test removed from `lock_cost`, the planted small walk granted the second lane while a large walk
waited for the first was named, and the test fails.

## 3. When the export's hold ended

`done` is written by `arms-held` inside the hold, and its seconds are the README's own clock from the
ask; the lanes go when the wrapper writes its line after it. So `arms` now ends the README with
`released <date time>, the lock's line: <the wrapper's line up to cmd=>` — the clock at the
wrapper's return, and the log's own `wait=`, `hold=`, `exit=`, `kind=` and `lane=`. The line is
found by the command as the wrapper words it, cut at its 200 characters, with the second the export
asked at; a README already carrying a `released` line was not written by this hold and is left alone,
and a hold on no line of the log is said.

## 4. What it costs

- The pair check cannot tell a ticketed wrapper's line from a racing one's. From batch seventy-six
  every wrapper in the worktree takes a ticket, so every pair is a defect there; a wrapper of another
  checkout (the main checkout before a merge) still races.
- `retake.py` is outside the tree and nothing gates it; the half of section 6 the lines alone answer
  is in `lock_cost`, which is.
- `released` lands a second or so after `done`. A wait on `^done` (habit 80) is unchanged.

## 5. Re-ask

After batch seventy-six's merge, `LOCK_COST_BATCH=batch-1480-1485 tools/state.sh gates-cost` and the
command it prints. A pair is a defect of ADR 1790; observed walks' queue above the race replay by more
than its error re-prices the order; C above S reopens the turn (ADR 1756 section 5).
